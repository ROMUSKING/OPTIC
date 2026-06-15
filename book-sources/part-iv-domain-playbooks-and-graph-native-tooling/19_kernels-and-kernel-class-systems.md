## 19. Kernels and Kernel-Class Systems

### 19.1 Why kernels are a meaningful stress test

A kernel is where the language's promises meet their harshest version.

- resources are linear or affine in the strongest possible sense,
- blocking mistakes can deadlock the system,
- latency and liveness rules matter,
- host boundaries are the whole game,
- provenance and observability are operational necessities rather than nice extras.

That makes kernels the hardest long-range test of the language's design.

### 19.2 Core kernel costates

#### 19.2.1 Representative kernel root decomposition

```text
KernelRuntime =
  FrameAllocator × AddressSpaces × RunQueues × DeviceRegistry × TraceBuffers
```

A kernel-class implementation naturally decomposes into costates such as:

- physical frame allocator,
- page tables and address spaces,
- run queues and scheduler state,
- IRQ or event routing structures,
- socket and channel tables,
- device and DMA descriptors,
- tracing buffers.

The point is not that the kernel becomes one giant lens. It becomes an explicit graph of transformations over explicit subsystems, each with strong ownership and timing rules.

### 19.3 Example: frame allocation as a linear optic

```rust
optic FrameAlloc: GradedOptic<FrameAllocator, PhysFrame,
    CacheGrade<2> + LinearGrade> {
    get  fa => choose_free_frame(fa)
    put  (fa, frame) => mark_allocated(fa, frame)
}
```

The key design claim is that a correctness property usually enforced by discipline or post hoc testing becomes part of the type and grade story: a frame is not just an integer index, it is a linear resource passing through a structured update path.

A frame allocator is a clean example because it makes the ownership story concrete. A frame must not be allocated twice, freed twice, or leaked silently.

```rust
optic FrameAlloc: GradedOptic<FrameAllocator, PhysFrame,
    CacheGrade<2> + LinearGrade> {
    get  fa => choose_free_frame(fa)
    put  (fa, frame) => mark_allocated(fa, frame)
}
```

The exact code shape may vary, but the point is stable: the type and grade structure are carrying a real systems invariant.

### 19.4 Page walks, interrupts, and scheduling

```rust
optic PageTableWalk: GradedOptic<AddressSpace, PhysFrame,
    CacheGrade<4> + LinearGrade> {
    get  as => walk_tables(as, va)
    put  (as, frame) => install_mapping(as, va, frame)
}
```

A four-level x86-64 walk is a nice pedagogical example because the grade is not an abstract number. It mirrors the hardware's own table depth almost exactly.

Page-table walks map neatly onto nested or composed optics because the hardware itself is a structured traversal through a fixed-depth hierarchy. Interrupt handling maps onto optics with hard non-blocking and latency grades. Scheduling maps onto optics over run queues and task state with liveness and ownership constraints.

This is where the book's insistence on explicit host context pays off. Hardware-visible and scheduler-visible state is not smuggled in through ambient calls. It is where the program actually says it is.

#### 19.4.1 Kernel reality: MMIO, DMA, interrupts, and assembly stay in the same graph

A real kernel does not live purely inside RAM-shaped data structures. It talks to devices through MMIO windows, submits DMA descriptors, acknowledges interrupts, manipulates page tables, fences memory, and occasionally uses privileged instructions or tiny sequences of assembly. The language only remains credible for kernel work if those operations fit the same summary model rather than exploding into ad hoc escape hatches.

The good news is that the current model is already close. An MMIO page is simply not `Ram<T>`; it is `Mmio<T>`. A descriptor ring shared with a NIC is not an ordinary queue; it is `Dma<DescriptorRing>`. An interrupt handler is not just another callback; it carries `ExecContext::Interrupt`, `BlockingGrade<Never>`, and a privilege requirement. Inline assembly is not a separate little language from the compiler's point of view; it is a boundary leaf with clobbers, volatility, and ordering constraints.

That reuse is what lets kernel code still benefit from the rest of the system.

- alias summaries continue to explain which page-table or queue regions are touched;
- grades continue to explain latency, blocking, ownership, and liveness constraints;
- determinism summaries continue to say which paths are replayable in simulation and which are not;
- diagnostics continue to point to one boundary declaration rather than to an opaque `unsafe` region spread across half a driver.

The hard rule is that the kernel subset should not get a shadow semantics. It gets stricter boundary contracts, stricter capabilities, and stricter execution-context rules, but it still lives in the same graph.

### 19.5 The kernel ladder

No serious project should jump from a Rust-hosted v0 compiler directly to a Linux-equivalent kernel claim. The ladder should be gradual.

1. user-space kernel simulator,
2. `no_std` runtime and allocator,
3. memory management and timers,
4. cooperative then preemptive scheduling,
5. block and network I/O,
6. SMP and capability/security layers.

The language earns each step only after the smaller invariants stay stable.

### 19.6 Transition

Kernels are the most austere systems domain. Browsers are nearly the opposite: huge structured dataflow systems with rendering, DOM mutation, incremental layout, and rich host interaction. The next chapter shows why the same core abstractions still apply.

### 19.7 Detailed implementation reference: kernel subsystems as costates and optics

Kernels are the harshest environment for the language’s promises because they expose real resource edges: frames, address spaces, interrupts, queues, devices, and scheduling deadlines. The detailed sections below make those mappings concrete.

A kernel's job is to mediate between hardware resources and processes. Every kernel subsystem manages a typed costate (a region of structured data) and exposes typed operations (optics) over it.

#### 19.7.1 Physical Memory Manager

```text
Costate:      FrameAllocator { free_list: BitSet, frames: SoA<FrameState> }
Focus type:   PhysFrame (a single 4KB page frame)
Optic:        FrameAlloc: GradedOptic<FrameAllocator, PhysFrame, LinearGrade + CacheGrade<2>>
```

The `LinearGrade` ensures each frame is allocated exactly once and freed exactly once. The type system prevents double-free and use-after-free at compile time: a `PhysFrame` value with `LinearGrade` must be passed to `FrameFree` before the scope exits, or the compiler emits `ALI-221`.

```rust
optic FrameAlloc: GradedOptic<FrameAllocator, PhysFrame, LinearGrade + CacheGrade<2>> {
    get  fa => {
        let idx = fa.free_list.find_first_set();  // bitset scan: CacheGrade<1>
        fa.free_list.clear(idx);                   // bitset write: CacheGrade<1>
        PhysFrame { idx }
    }
    put  (fa, frame) => { fa.free_list.set(frame.idx); }  -- this is actually FrameFree
}
```

The grade `CacheGrade<2>` reflects: one cache line for the bitset (free list), one for the frame state array. On a 64-bit bitset with 64 entries per word, `find_first_set` touches exactly one 64-byte cache line.

#### 19.7.2 Virtual Memory and Page Table Walks

A page table walk in x86-64 is a four-level traversal: PML4 → PDPT → PD → PT → physical frame. This is naturally a `Traversal` or nested `Compose`:

```rust
optic PageTableWalk: GradedOptic<AddressSpace, PhysFrame,
    CacheGrade<4> + LinearGrade + IOGrade<0ns>>
{
    -- get: walk the table, load the terminal PTE
    get  as =>
        as.pml4[va.pml4_idx()]
          .pdpt[va.pdpt_idx()]
          .pd[va.pd_idx()]
          .pt[va.pt_idx()]
          .frame()

    -- put: update the terminal PTE
    put  (as, frame) => {
        as.pml4[va.pml4_idx()]
          .pdpt[va.pdpt_idx()]
          .pd[va.pd_idx()]
          .pt[va.pt_idx()]
          .set_frame(frame);
        flush_tlb(va);
    }
}
```

`CacheGrade<4>` is exact: four cache lines, one per table level. `IOGrade<0ns>` confirms this is purely in-memory (no I/O). The `LinearGrade` ensures the frame is not aliased into multiple page table entries.

#### 19.7.3 Interrupt Handler Discipline

An interrupt handler in a kernel context has two requirements: it must be fast (bounded latency), and it must not block (no sleep, no mutex acquisition that could deadlock with the interrupted code).

The grade system enforces these via `LatencyGrade` and `BlockingGrade`:

```rust
optic IRQHandler: GradedOptic<IRQState, IRQEvent,
    LatencyGrade<5us> + BlockingGrade<NonBlocking> + LinearGrade>
{
    get  irq => irq.pending_events.pop_front()
    put  (irq, event) => irq.handled.push_back(event)
}
```

`BlockingGrade<NonBlocking>` is checked by the compiler: any optic body that could block (mutex lock, channel receive, I/O wait) inside a `NonBlocking`-graded optic is a `KRN-1xx` error. The compiler's optic body analyzer detects:
- Any `put_reads` or `put_writes` to a `Mutex<T>` costate
- Any call to a function with `BlockingGrade<MayBlock>`
- Any coinductive optic call without `LivenessGrade<Bounded>`

#### 19.7.4 Scheduler as Optic

The process scheduler manages a `RunQueueSet` costate. Scheduling decisions are optics over that costate:

```rust
optic CFS_Dequeue: GradedOptic<RunQueueSet, Task,
    CacheGrade<3> + LinearGrade + LatencyGrade<1us>>
{
    get  rqs => {
        let cpu = rqs.current_cpu();
        rqs.per_cpu[cpu].run_queue.min_vruntime_task()  // O(log n) red-black tree
    }
    put  (rqs, task) => {
        let cpu = rqs.current_cpu();
        rqs.per_cpu[cpu].run_queue.remove(task);
        rqs.per_cpu[cpu].current = Some(task);
    }
}
```

The composition `CFS_Dequeue >>> ContextSwitch` is the full scheduler tick: dequeue the next task, then switch to it. The grade algebra computes the total latency:

```text
combine_seq(LatencyGrade<1us>, LatencyGrade<2us>) = LatencyGrade<3us>
```

If the platform has a hard real-time budget of 5µs per tick, the compiler verifies `3 ≤ 5` and accepts the composition. An over-budget composition is a compile-time error, not a runtime overrun.

---

