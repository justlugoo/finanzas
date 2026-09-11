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

La app aparece en el menú de aplicaciones y en el system tray al iniciar sesión si el autoarranque está activado desde Configuración.

---

## Primera vez

Al abrir la app por primera vez estará completamente vacía. Flujo recomendado:

1. **Configuración → Vehículos** — agrega al menos un vehículo: nombre, rendimiento (km/galón) y capacidad del tanque (galones). Ambos son obligatorios — la capacidad no es solo para mostrar el nivel y la autonomía en el Dashboard, también permite que la app te avise si el rendimiento quedó mal configurado.
2. **Configuración → Gasolina** — registra el precio actual del galón.
3. **Configuración → Presupuestos** — crea tus categorías de ingreso y gasto, con su meta mensual.
4. **Configuración → Gasolina → Costos por ruta** — agrega las rutas que recorres frecuentemente (km ida y vuelta), si quieres ver el costo estimado por trayecto.
5. **Registrar** — ya puedes registrar ingresos, gastos y tanqueos.
6. **Metas** — desde aquí, no desde Registrar, se crean ahorros, préstamos y deudas (incluyendo compras a crédito).

---

## Pantallas

| Pantalla | Descripción |
|----------|-------------|
| **Resumen** | Dashboard con disponible y patrimonio siempre visibles, filtro de período (Diario/Semanal/Mensual/Anual/Total — Mensual por defecto), progreso de presupuestos por categoría (la meta se ajusta automáticamente al período elegido), comparativa con el mes anterior, últimas transacciones, objetivos pendientes y nivel de gasolina por vehículo. |
| **Registrar** | Tres modos: Ingreso, Gasto y Tanqueo. Categoría por chips, fecha, nota, marca de gasto extraordinario, y kilometraje opcional (registra el consumo de un viaje, sin costo asociado). El tanqueo pide el precio del galón (editable) y registra el gasto real y el tanqueo juntos. Un gasto que supera el disponible no se guarda — si necesitas financiarlo, se registra como deuda desde Metas. |
| **Historial** | Lista agrupada por día (con año visible), filtrable por período (incluyendo "Total", desde el primer registro), tipo, categoría y texto en notas. Edición y eliminación inline, selección múltiple con borrado masivo, exportación a CSV. |
| **Metas** | Vista unificada de préstamos por cobrar, deudas por pagar y objetivos de ahorro, ordenados por avance. Abonos parciales, racha de meses seguidos abonando, cuenta regresiva de días para metas de ahorro con fecha objetivo, y sección de logros para lo ya completado. |
| **Configuración** | Organizada en pestañas: Gasolina (precio, costos por ruta, nivel de tanque, historial de precios), Vehículos, Presupuestos (categorías + montos mensuales), Sistema (autoarranque, backup) y Datos (restablecimiento de fábrica). |

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

El backup se exporta desde **Configuración → Sistema**.

---

## Licencia

[MIT](LICENSE.md)
