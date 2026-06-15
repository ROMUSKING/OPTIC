## 20. Browsers and Interactive Rendering Systems

### 20.1 Why browsers fit the model

A browser is a long pipeline of transformations over structured data.

- parse HTML into DOM-like structures,
- cascade style,
- measure and place layout boxes,
- build paint and compositing data,
- rasterize and present,
- feed events back into document state.

This is almost the ideal demonstration that the language is not just for kernels or ECS loops.

### 20.2 DOM as a structured costate

```rust
data Document {
    nodes:          SoA<DomNode>,
    css_properties: SoA<ComputedStyle>,
    layout_boxes:   SoA<LayoutBox>,
    paint_layers:   SoA<PaintLayer>,
}
```

A browser does not need to use this exact layout everywhere, but the hot paths benefit enormously when DOM-adjacent data stops being a pointer-rich object graph and becomes an explicit structured arena.

A browser implementation can choose to model DOM nodes, computed styles, layout boxes, paint layers, and event registrations as typed arrays keyed by node id rather than as pointer-heavy object graphs in the hot path.

That makes style resolution and layout natural traversal and composition problems instead of opaque tree walkers.

### 20.3 Two-pass layout as explicit sequential composition

```rust
let layout_pipeline = MeasurePass >>> PlacementPass;
```

This tiny line is representative of the language's whole strategy. A common browser-engine fact becomes an explicit composition node that can carry cache, latency, staging, and provenance information instead of being hidden in a call stack.

Layout is a strong example of why sequential composition should stay explicit.

- first pass: intrinsic measurement,
- second pass: placement given containing blocks and constraints.

Treating that as `MeasurePass >>> PlacementPass` keeps the budget, fusion, and staging story honest. The compiler can see that this is two related passes over the same world rather than two arbitrary functions that happen to be called in sequence.

### 20.4 Rendering as staged and coinductive structure

```rust
stage {
    let visible_meshes = FrustumCull >>> SortByDepth >>> BuildDrawCall;
}

frame_buffer
    .query(visible_meshes)
    .coinductive()
    .drive();
```

The important part of the example is not the exact names of the passes. It is the separation between compile-or-frame-time specialization and live repeated execution.

Rendering is exactly the kind of problem that benefits from staged precomputation and then repeated event-driven execution.

- stage scene- or frame-level plans,
- coinductively drive input, animation, layout invalidation, and raster updates,
- keep provenance and diagnostics over the pipeline graph.

This is also one of the clearest examples of why observability as graph nodes is valuable. Browsers are notoriously difficult to debug once optimization, caching, and incremental invalidation are in play. A language-aware graph view is an unusually strong fit here.

### 20.5 Transition

Databases and games push on different aspects of the same architecture: one emphasizes plans, indexes, and transactional boundaries; the other emphasizes bulk data, SIMD, and frame budgets.

### 20.6 Detailed implementation reference: DOM, style, layout, and rendering as structured pipelines

Browser engines are full of repeated passes over large, heterogeneous state. The supplement below shows how the language forces those passes into explicit, checkable structures rather than allowing them to disappear into ad hoc graph walks and callback layers.

A web browser is essentially a complex data transformation pipeline from HTML/CSS source to rasterized pixels. Each stage in the pipeline is a natural optic over a typed costate.

#### 20.6.1 The DOM as a Costate

The Document Object Model is the browser's central costate. Each node is a focus; CSS properties, layout boxes, and event handlers are separate SoA fields hung off the same node IDs:

```rust
data Document {
    nodes:          SoA<DomNode>,       -- tag, id, class list
    css_properties: SoA<ComputedStyle>, -- resolved CSS values
    layout_boxes:   SoA<LayoutBox>,     -- position, size, overflow
    paint_layers:   SoA<PaintLayer>,    -- compositing layer assignment
    event_handlers: SoA<HandlerList>,   -- registered event listeners
}
```

This is exactly SoA layout for a DOM: instead of a pointer-chained tree of heterogeneous nodes (the traditional implementation), all nodes are stored in flat arrays indexed by node ID. CSS selectors become queries over `css_properties`; layout is a traversal over `layout_boxes`; painting is a traversal over `paint_layers`.

#### 20.6.2 Style Resolution as a Traversal

CSS cascading resolves computed styles from inherited rules, author stylesheets, and user-agent defaults:

```rust
optic StyleResolution: GradedTraversal<Document, ComputedStyle,
    CacheGrade<4> + SharedGrade + LatencyGrade<100us>>
{
    traverse doc => doc.css_properties.iter_mut()
        .zip(doc.nodes.iter())
        .filter(|(_, node)| node.is_element())
    update (doc, resolved_styles) =>
        doc.css_properties.iter_mut().zip(resolved_styles).for_each(|(cs, rs)| *cs = rs)
}
```

The traversal over all DOM elements respects selector specificity and inheritance order. Because `SharedGrade` is used (no in-place mutation during traversal), the traversal can be parallelized across subtrees of the DOM tree that have no CSS-inheritance dependency between them.

#### 20.6.3 Layout as Sequential Composition

Layout in a browser is inherently two-pass: first measure the intrinsic sizes of all nodes (bottom-up), then place them based on the containing block geometry (top-down). This is `MeasurePass >>> PlacementPass`:

```rust
optic MeasurePass: GradedOptic<Document, SizedNode, CacheGrade<3> + AffineGrade> {
    get  doc => measure_intrinsic(doc.nodes, doc.css_properties)
    put  (doc, sizes) => { doc.layout_boxes = sizes; }
}

optic PlacementPass: GradedOptic<Document, PlacedNode, CacheGrade<2> + AffineGrade> {
    get  doc => place_in_containing_block(doc.layout_boxes, doc.viewport)
    put  (doc, placed) => { doc.layout_boxes = placed; }
}

let layout_pipeline = MeasurePass >>> PlacementPass;
```

The grade of the composed pipeline is `CacheGrade<5>` (3 + 2) and `LatencyGrade<frame_budget - style_budget>`. If the composed grade exceeds the frame budget, the compiler requires splitting the pipeline and running the two passes in separate frame phases.

#### 20.6.4 The Rendering Pipeline as a Staged Optic

The rendering pipeline from layout to rasterized pixels is a natural staged optic. The layout boxes are known at "layout time" (before paint); the rasterization happens "paint time":

```rust
stage {
    -- Layout-time: compute which elements need their own compositing layer
    let layer_assignment = PaintLayerOptic >>> CompositingDecision;
}

-- Paint-time: rasterize using the staged (pre-computed) layer decisions
frame_buffer.query(RasterizeLayer(layer_assignment)).drive();
```

The `stage { }` block ensures that layer decisions (which are expensive to recompute) are cached and reused across frames unless the DOM changes. The grade algebra tracks which fields are accessed in the staged portion vs. the live paint loop, preventing the staged optic from accidentally reading live paint data.

---

