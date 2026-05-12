# Finanzas

App de escritorio para gestión financiera personal. Registra ingresos y gastos, controla presupuestos por categoría, sigue objetivos de ahorro y calcula costos de gasolina por vehículo y ruta.

**Plataforma:** Linux (Fedora / Debian / Ubuntu)  
**Idioma:** Español  
**Tema:** Oscuro fijo

---

## Instalación

Descarga el paquete de la sección [Releases](../../releases) del repositorio:

- **Fedora / openSUSE:** `Finanzas-x.x.x-1.x86_64.rpm`
- **Debian / Ubuntu:** `finanzas_x.x.x_amd64.deb`

```bash
# Fedora
sudo rpm -i Finanzas-*.rpm

# Debian / Ubuntu
sudo dpkg -i finanzas_*.deb
```

La app aparece en el menú de aplicaciones y en el system tray al iniciar sesión si el autoarranque está activado desde Configuración.

---

## Primera vez

Al abrir la app por primera vez estará completamente vacía. Flujo recomendado:

1. **Configuración → Vehículos** — agrega al menos un vehículo con su rendimiento en km/galón.
2. **Configuración → Gasolina** — registra el precio actual del galón.
3. **Configuración → Presupuestos mensuales** — crea tus categorías de ingreso y gasto con sus metas.
4. **Configuración → Costos por ruta** — agrega las rutas que recorres frecuentemente (km ida y vuelta).
5. **Registrar** — ya puedes registrar tu primer movimiento.

---

## Pantallas

| Pantalla | Descripción |
|----------|-------------|
| **Resumen** | Dashboard: saldo, KPIs del mes, progreso por categoría, comparativa con el mes anterior, últimas transacciones y próximo objetivo |
| **Registrar** | Formulario para ingresos y gastos. Soporta categoría, monto, fecha, nota, gasto extraordinario, objetivo asociado y km recorridos con selector de vehículo |
| **Historial** | Lista filtrable por período, tipo y categoría. Edición y eliminación inline, selección múltiple y exportación a CSV |
| **Objetivos** | Metas de ahorro y deudas con progreso, monto mensual requerido y fecha estimada de cumplimiento |
| **Configuración** | Categorías, vehículos, rutas, precio de gasolina, autoarranque, backup y restablecimiento de fábrica |

---

## Desarrollo

### Requisitos

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 20+
- [pnpm](https://pnpm.io/installation)
- Dependencias de sistema para Tauri:

```bash
# Fedora
sudo dnf install webkit2gtk4.1-devel openssl-devel

# Debian / Ubuntu
sudo apt install libwebkit2gtk-4.1-dev libssl-dev
```

### Correr en desarrollo

```bash
git clone <url-del-repo>
cd Finanzas
pnpm install
pnpm tauri dev
```

### Compilar release

```bash
pnpm tauri build
# Genera .rpm y .deb en src-tauri/target/release/bundle/
```

### Otros comandos

```bash
pnpm check    # Type-checking (svelte-check + tsc)
pnpm dev      # Solo el frontend Vite (sin Tauri)
pnpm build    # Solo el build del frontend
```

---

## Base de datos

SQLite local, sin dependencias externas ni sincronización cloud.  
**Ubicación:** `~/.local/share/finanzas/local.db`

El backup se exporta desde **Configuración → Base de datos local**.

---

## Licencia

[MIT](LICENSE.md)
