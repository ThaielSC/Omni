# Ramas virtuales de GitButler para Omni

Omni usa el CLI `but` y ramas virtuales apiladas. No usar comandos Git de escritura (`git add`, `git commit`, `git checkout`, `git push`) para administrar este stack.

## Stack actual

```text
feat/docs
└─ feat/models-config
   └─ feat/e2e-test-suite
      └─ feat/providers-import
         └─ feat/tui
```

Las ramas aparecen simultáneamente en el workspace GitButler; cada rama hija se apoya en las anteriores.

## Responsabilidades

1. `feat/docs`: especificaciones, roadmap y documentación base.
2. `feat/models-config`: modelos unificados, configuración TOML y utilidades.
3. `feat/e2e-test-suite`: fixtures y pruebas de CLI/core.
4. `feat/providers-import`: providers nativos y dinámicos, CLI texto/JSON y clasificación `InstallReason`.
5. `feat/tui`: interfaz Ratatui, navegación, statusline, búsqueda, filtro de dependencias y temas/layout configurables.

## Flujo de trabajo

- Revisar el orden y las asignaciones con `but status`; para obtener IDs de archivos usa `but status -fv` o `but diff`.
- Crear commits selectivos con `but commit -b <rama> -m "<mensaje>" <ids>`.
- Para apilar una rama existente se usa `but move <hija> --above <padre>`.
- Sincronizar el workspace con `but pull`; publicar y crear PRs con `but push` / `but pr new` según el flujo de la skill local `but`.
- Las operaciones de desinstalación pasan por confirmación en la TUI; los cambios privilegiados requieren autorización previa (`sudo -v` con sudo).
