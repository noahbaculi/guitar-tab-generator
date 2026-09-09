# Where demo_mockup.png comes from

`examples/demo_mockup.png`, the demo image in the README, is generated in the website repo, not here. It composites a capture of the live demo page into a MacBook render.

Regenerate it with the recipe at `noahbaculi.github.io/docs/screenshots/guitar-tab-mockup-recipe.md`. That repo runs the demo page, holds the mockup art, and owns the responsive webp variants the site serves, so the recipe lives there and the last step copies the result back into `examples/`.
