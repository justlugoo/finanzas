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

1. **Configuración → Vehículos** — agrega al menos un vehículo con su rendimiento en km/galón. Opcionalmente añade la capacidad del tanque en litros para ver el nivel y la autonomía en el dashboard.
2. **Configuración → Gasolina** — registra el precio actual del galón. Necesario antes de registrar viajes o tanqueos.
3. **Configuración → Presupuestos mensuales** — crea tus categorías de ingreso y gasto con sus metas.
4. **Configuración → Costos por ruta** — agrega las rutas que recorres frecuentemente (km ida y vuelta).
5. **Registrar** — ya puedes registrar ingresos, gastos y tanqueos.

---

## Pantallas

| Pantalla | Descripción |
|----------|-------------|
| **Resumen** | Dashboard: saldo en mano, patrimonio (con préstamos pendientes), KPIs del mes, progreso por categoría, comparativa con el mes anterior, últimas transacciones y widget de nivel de gasolina por vehículo |
| **Registrar** | Tres modos: Ingreso, Gasto y Tanqueo. Soporta categoría, monto, fecha, nota, gasto extraordinario, cuotas estimadas, objetivo asociado y km recorridos con selector de vehículo |
| **Historial** | Lista filtrable por período, tipo y categoría. Edición y eliminación inline, selección múltiple y exportación a CSV |
| **Metas** | Vista unificada de préstamos por cobrar, deudas por pagar y objetivos de ahorro. Abonos parciales, edición, eliminación, badges de progreso, estado automático y fecha estimada de cumplimiento |
| **Configuración** | Categorías, vehículos (con capacidad de tanque), rutas, precio de gasolina, autoarranque, backup y restablecimiento de fábrica |

---

## Desarrollo

```bash
git clone <url-del-repo>
cd Finanzas
pnpm install
pnpm tauri dev   # inicia la app en modo desarrollo con hot-reload
```

### Otros comandos

```bash
pnpm check    # type-checking (svelte-check + tsc)
pnpm dev      # solo el frontend Vite (sin Tauri)
pnpm build    # solo el build del frontend
```

---

## Base de datos

SQLite local, sin dependencias externas ni sincronización cloud.  
**Ubicación:** `~/.local/share/finanzas/local.db`

El backup se exporta desde **Configuración → Base de datos local**.

---

## Licencia

[MIT](LICENSE.md)
