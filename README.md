# Finanzas

App de escritorio para gestión financiera personal. Registra ingresos y gastos, controla presupuestos por categoría, sigue objetivos de ahorro, gestiona préstamos a terceros y calcula costos de gasolina por vehículo y ruta.

**Plataforma:** Linux (Fedora / Debian / Ubuntu)  
**Idioma:** Español  
**Tema:** Oscuro fijo

---

## Instalación

### Opción A — Paquete precompilado (recomendado)

Descarga el paquete de la sección [Releases](../../releases) del repositorio:

- **Fedora / openSUSE:** `Finanzas-x.x.x-1.x86_64.rpm`
- **Debian / Ubuntu:** `finanzas_x.x.x_amd64.deb`

> **Versiones mínimas soportadas:**
> - Fedora 37 o superior (recomendado 40+)
> - Debian 12 (Bookworm) o superior
> - Ubuntu 22.04 LTS (Jammy) o superior
> - Linux Mint 21+, Pop!_OS 22.04+, Elementary OS 7+ (derivados de Ubuntu 22.04+)
>
> El requisito real es que la distribución provea `webkit2gtk` versión 4.1 en sus repositorios oficiales. Para Arch Linux, openSUSE, NixOS, Gentoo u otras distribuciones sin paquete precompilado, compila desde el código fuente (Opción B).

```bash
# Fedora
sudo dnf install ./Finanzas-*.rpm

# Debian / Ubuntu
sudo apt install ./finanzas_*.deb
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
sudo dnf install ./src-tauri/target/release/bundle/rpm/Finanzas-*.rpm

# Debian / Ubuntu
sudo apt install ./src-tauri/target/release/bundle/deb/finanzas_*.deb
```

La app aparece en el menú de aplicaciones y en el system tray al iniciar sesión si el autoarranque está activado desde Configuración.

---

## Primera vez

Al abrir la app por primera vez estará completamente vacía. Flujo recomendado:

1. **Configuración → Vehículos** — agrega al menos un vehículo con su rendimiento en km/galón.
2. **Configuración → Gasolina** — registra el precio actual del galón. Necesario antes de registrar transacciones con km recorridos — el cálculo automático de costo de gasolina lo requiere.
3. **Configuración → Presupuestos mensuales** — crea tus categorías de ingreso y gasto con sus metas.
4. **Configuración → Costos por ruta** — agrega las rutas que recorres frecuentemente (km ida y vuelta).
5. **Registrar** — ya puedes registrar tu primer movimiento.

---

## Pantallas

| Pantalla | Descripción |
|----------|-------------|
| **Resumen** | Dashboard: saldo en mano, patrimonio (cuando hay préstamos pendientes), KPIs del mes, progreso por categoría, comparativa con el mes anterior y últimas transacciones |
| **Registrar** | Formulario para ingresos y gastos. Soporta categoría, monto, fecha, nota, gasto extraordinario, objetivo asociado y km recorridos con selector de vehículo |
| **Historial** | Lista filtrable por período, tipo y categoría. Edición y eliminación inline, selección múltiple y exportación a CSV |
| **Objetivos** | Metas de ahorro y deudas con progreso, monto mensual requerido y fecha estimada de cumplimiento |
| **Préstamos** | Dinero prestado a terceros: registra deudor, monto y fecha; abonos parciales con transición automática a "pagado"; saldo pendiente por cobrar |
| **Configuración** | Categorías, vehículos, rutas, precio de gasolina, autoarranque, backup y restablecimiento de fábrica |

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
