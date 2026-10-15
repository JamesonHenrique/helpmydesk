# HelpMyDesk — protótipo da interface

Interface Angular das telas do plano v1 (III.13). Dentro do app desktop (Tauri) ela fala com o
núcleo em Rust por `CoreApi` → `TauriApi`; aberta num navegador, usa dados de exemplo
(`MockApi`) e **não verifica nem altera nada** no computador.

Os textos vêm do catálogo revisado, gerados por:

```sh
cargo run -q -p hmd-core --example ui_fixtures > apps/desktop/ui/src/app/data/catalog.generated.ts
```

O CI falha se o arquivo gerado estiver desatualizado.

```sh
cd apps/desktop/ui
npm ci --ignore-scripts
npx ng serve        # http://localhost:4200
npx ng test --watch=false
npx ng build
```

Versões fixas, todas publicadas há pelo menos duas semanas quando adicionadas (Angular 22.2.1).
