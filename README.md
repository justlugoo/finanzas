# FinCapX

App de escritorio para gestión financiera personal. Registra ingresos, gastos y tanqueos; controla presupuestos por categoría; unifica objetivos de ahorro, deudas y préstamos en un módulo de Metas; y hace seguimiento del nivel de gasolina del tanque con autonomía estimada por vehículo.

**Plataforma:** Linux (Fedora / Debian / Ubuntu)
**Idioma:** Español
**Tema:** Oscuro fijo

---

## Instalación

### Opción A — Paquete precompilado (recomendado)

Descarga el paquete de la sección [Releases](../../releases) del repositorio:

- **Fedora / openSUSE:** `FinCapX-x.x.x-1.x86_64.rpm`
- **Debian / Ubuntu:** `fincapx_x.x.x_amd64.deb`

> **Versiones mínimas soportadas:**
> - Fedora 37 o superior (recomendado 40+)
> - Debian 12 (Bookworm) o superior
> - Ubuntu 22.04 LTS (Jammy) o superior
> - Linux Mint 21+, Pop!_OS 22.04+, Elementary OS 7+ (derivados de Ubuntu 22.04+)
>
> El requisito real es que la distribución provea `webkit2gtk` versión 4.1 en sus repositorios oficiales. Para Arch Linux, openSUSE, NixOS, Gentoo u otras distribuciones sin paquete precompilado, compila desde el código fuente (Opción B).

```bash
# Fedora
sudo dnf install ./FinCapX-*.rpm

# Debian / Ubuntu
sudo apt install ./fincapx_*.deb
```

### Opción B — Compilar desde el código fuente

**Requisitos:**

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 20+
- [pnpm](https://pnpm.io/installation)
- Dependencias de sistema:

```bash
# Fedora
sudo dnf install webkit2gtk4.1-devel openssl-devel

# Debian / Ubuntu
sudo apt install libwebkit2gtk-4.1-dev libssl-dev
```

```bash
git clone <url-del-repo>
cd Finanzas
pnpm install
pnpm tauri build
# Genera .rpm y .deb en src-tauri/target/release/bundle/
```

```bash
# Fedora
sudo dnf install ./src-tauri/target/release/bundle/rpm/FinCapX-*.rpm

# Debian / Ubuntu
sudo apt install ./src-tauri/target/release/bundle/deb/fincapx_*.deb
```

La app aparece en el menú de aplicaciones y en el system tray al iniciar sesión si el autoarranque está activado desde Ajustes.

---

## Primera vez

Al abrir la app por primera vez (o después de actualizar a una versión nueva) aparece un tour guiado opcional: un recorrido informativo por las pantallas esenciales, con un mensaje breve anclado sobre el elemento real de cada pantalla — no bloquea nada ni exige crear datos, y se puede omitir en cualquier momento. Vuelve a aparecer en cada versión nueva, pero nunca por "Restablecer datos de fábrica" (ese reinicio no te vuelve nuevo en la app).

Un vehículo **no es obligatorio** para usar la app — si no registras ninguno, Registros simplemente no muestra Tanqueo ni Kilometraje (no aplica). Flujo recomendado:

1. **Ajustes → Presupuestos** — crea tus categorías de ingreso y gasto, con su meta mensual.
2. **Ajustes → Vehículos y gasolina** (opcional, solo si tienes vehículo) — agrega el vehículo (nombre, rendimiento en km/galón y capacidad del tanque en galones, ambos obligatorios), el precio actual del galón, y de paso las rutas que recorres frecuentemente si quieres ver el costo estimado por trayecto.
3. **Registros** — ya puedes registrar ingresos, gastos y (si tienes vehículo) tanqueos.
4. **Historial** — revisa y filtra todo lo registrado.
5. **Metas** — desde aquí, no desde Registros, se crean ahorros, préstamos y deudas (incluyendo compras a crédito).

---

## Pantallas

| Pantalla | Descripción |
|----------|-------------|
| **Resumen** | Dashboard con disponible y patrimonio siempre visibles, filtro de período (Diario/Semanal/Mensual/Anual/Total — Mensual por defecto), progreso de presupuestos por categoría (la meta se ajusta automáticamente al período elegido), comparativa con el mes anterior, últimas transacciones, objetivos pendientes y nivel de gasolina por vehículo (si aplica). |
| **Registros** | Ingreso, Gasto y (si hay al menos un vehículo registrado) Tanqueo. Categoría por chips, fecha, nota, marca de gasto extraordinario, y kilometraje opcional (registra el consumo de un viaje, sin costo asociado). El tanqueo pide el precio del galón (editable) y registra el gasto real y el tanqueo juntos. Un gasto que supera el disponible no se guarda — si necesitas financiarlo, se registra como deuda desde Metas. El formulario conserva lo que llevas escrito si cambias de pantalla y vuelves; un botón "Limpiar formulario" lo vacía a propósito. |
| **Historial** | Lista agrupada por día (con año visible), filtrable por período (incluyendo "Total", desde el primer registro), tipo, categoría y texto en notas. Edición y eliminación inline, selección múltiple con borrado masivo, exportación a CSV. Una categoría eliminada con movimientos asociados se archiva (no se borra) y se distingue con una etiqueta punteada "Archivada". |
| **Metas** | Vista unificada de préstamos por cobrar, deudas por pagar y objetivos de ahorro, ordenados por avance. Abonos parciales, racha de meses seguidos abonando, cuenta regresiva de días para metas de ahorro con fecha objetivo, y sección de logros para lo ya completado. |
| **Ajustes** | Pestañas: **Presupuestos** (categorías + montos mensuales, con renombrar y archivar en vez de bloquear el borrado), **Vehículos y gasolina** (vehículos, precio del galón, costos por ruta, nivel de tanque con vista previa antes de resetear, historial de precios), **Sistema** (autoarranque, backup) y **Datos** (restablecimiento de fábrica). |

---

## Desarrollo

```bash
git clone <url-del-repo>
cd Finanzas
pnpm install
pnpm tauri dev   # inicia la app en modo desarrollo con hot-reload
```

Comandos disponibles y convenciones de código: ver [`CLAUDE.md`](CLAUDE.md). Referencia técnica completa (stack, modelo de datos, comandos Tauri, estructura de carpetas): ver [`docs/architecture.md`](docs/architecture.md).

---

## Base de datos

SQLite local, sin dependencias externas ni sincronización cloud.
**Ubicación:** `~/.local/share/finanzas/local.db`

El backup se exporta desde **Ajustes → Sistema**.

---

## Licencia

[MIT](LICENSE.md)
