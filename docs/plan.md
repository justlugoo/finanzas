# Plan — Finanzas

App de escritorio multiplataforma (Linux + Windows) con backend Rust, sincronización cloud transparente vía Turso, ejecución continua en segundo plano y notificaciones nativas. Cero motores de DB instalados localmente.

---

## 1. Objetivo

Gestionar finanzas personales con observabilidad real: registrar movimientos, comparar contra metas estimadas y contra histórico, detectar desviaciones, planear objetivos a futuro. Sin internet la app funciona al 100%; sincroniza al reconectar.

---

## 2. Principios

1. **Local-first con sync cloud transparente.**
2. **Una pantalla, una intención.** Sin sidebars saturados.
3. **Segundo plano por defecto.** La ventana se abre solo cuando se necesita.
4. **Reversible.** Todo lo que se hace se puede deshacer.
5. **Una sola forma de hacer cada cosa.** Sin abstracciones especulativas.

---

## 3. Alcance funcional

### Lo que SÍ hace

| ID | Descripción |
|---|---|
| RF-01 | CRUD de ingresos y gastos |
| RF-02 | Reflejo automático en vistas día / semana / mes / año |
| RF-03 | Comparativa de gastos vs meta estimada por categoría con % de ejecución |
| RF-04 | Comparativa mes actual vs mes anterior con % de variación |
| RF-05 | Marcado de gastos como extraordinarios (no penalizan ejecución de meta) |
| RF-06 | Categoría de ingresos eventuales / varios |
| RF-07 | Notificación nativa cuando se excede meta de categoría |
| RF-08 | Registro manual del precio de gasolina |
| RF-09 | Scraping opcional del precio de gasolina (1 fuente, fallback manual) |
| RF-10 | Histórico de precios y comparativa semanal |
| RF-11 | Cálculo automático del costo por viaje según km y precio actual |
| RF-12 | Objetivos de ahorro con monto, fecha objetivo y progreso |
| RF-13 | Historial filtrable por período y categoría |
| RF-14 | Edición de metas y parámetros desde la app |
| RF-15 | Sincronización entre Linux y Windows vía Turso embedded replicas |
| RF-16 | Ejecución en segundo plano con system tray |
| RF-17 | Exportar historial a CSV |

### Lo que NO hace

- Sin migraciones formales (un `schema.sql` al iniciar).
- Sin auth multiusuario.
- Sin atajos de teclado.
- Sin múltiples cuentas, monedas o usuarios.
- Sin app móvil.
- Sin reportes PDF.
- Sin tabla de log de notificaciones.

---

## 4. Requerimientos no funcionales

| Atributo | Requisito |
|---|---|
| Plataformas | Linux (AppImage) + Windows (MSI) |
| Memoria en idle (segundo plano, single-process) | 25–50 MB |
| Memoria con ventana abierta | 80–200 MB |
| Tiempo de arranque | < 1.5s |
| Tamaño binario | < 25 MB |
| DB engine local instalado | Cero |
| Funcionamiento sin internet | Sí; sync se difiere |
| Idioma | Español |
| Tema | Oscuro siempre |

**Nota sobre memoria.** Los 25–50 MB en idle son del WebView del sistema, no de Rust. Si en uso real el consumo molesta, se evalúa en fase de Operación una arquitectura de dos procesos: daemon ligero (5–10 MB) siempre activo + ventana Tauri spawneada bajo demanda.

---

## 5. Stack técnico (confirmado)

| Capa | Tecnología | Justificación |
|---|---|---|
| Framework desktop | **Tauri 2.x** | Binarios livianos, backend Rust nativo, system tray y notificaciones built-in, multiplataforma sin esfuerzo |
| Backend lógico | **Rust 1.75+** | Requisito explícito; rendimiento, type safety |
| Frontend | **Svelte 5** | Compila a JS vanilla sin runtime, sintaxis mínima, bundle pequeño |
| Estilos | **CSS puro + variables** | Sin Tailwind ni preprocesadores. Tema oscuro fijo, control total |
| DB | **Turso (libSQL)** | SQLite-compatible, sync cloud nativo, free tier suficiente, sin instalación local |
| Capa DB Rust | **libsql** (crate oficial) | Soporte de embedded replicas + sync |
| Scraping | **reqwest + scraper** | Estándar Rust para HTTP + parsing HTML |
| Notificaciones | **tauri-plugin-notification** | API nativa multiplataforma |
| System tray | **tauri-plugin-tray** | Built-in en Tauri 2.x |
| Autoarranque | **tauri-plugin-autostart** | Inicio automático con sesión del SO |
| Empaquetado | **tauri build** | AppImage Linux + MSI Windows en un comando |

---

## 6. Arquitectura

```
┌─────────────────────────────────────────────┐
│              Tauri App (Finanzas)           │
│                                             │
│  ┌─────────────────┐   ┌───────────────┐    │
│  │  Frontend       │◄─►│  Backend Rust │    │
│  │  Svelte         │   │  (commands)   │    │
│  └─────────────────┘   └───────┬───────┘    │
│                                │            │
│  ┌─────────────────────────────▼─────────┐  │
│  │  libSQL embedded replica (.db)        │  │
│  │  Linux: ~/.local/share/finanzas/      │  │
│  │  Win: %APPDATA%\finanzas\             │  │
│  └─────────────────┬─────────────────────┘  │
└────────────────────│────────────────────────┘
                     │ sync async
                     ▼
              ┌──────────────┐
              │ Turso Cloud  │
              └──────────────┘
                     ▲
                     │ sync async
       ┌─────────────┴──────────────┐
       │  Otra máquina (Linux/Win)  │
       └────────────────────────────┘
```

El frontend Svelte llama comandos Rust mediante `invoke` de Tauri. El backend Rust lee/escribe en la replica local (instantáneo). La sincronización con Turso ocurre en background después de cada escritura y al iniciar la app.

---

## 7. Modelo de datos

5 tablas. Categorías como strings directos en `transactions` y `budgets`. Aportes a objetivos derivan de `transactions.goal_id`. Sin tabla de log de notificaciones.

### `transactions`
| Campo | Tipo | Descripción |
|---|---|---|
| id | INTEGER PK | Autoincremental |
| date | TEXT | ISO date YYYY-MM-DD |
| type | TEXT | `ingreso` o `gasto` |
| category | TEXT | Nombre directo de categoría |
| amount | INTEGER | Valor en COP |
| note | TEXT | Nullable |
| is_extraordinary | INTEGER | 0 o 1 |
| goal_id | INTEGER | Nullable, FK a `goals` |
| created_at | TEXT | ISO timestamp |

### `budgets`
| Campo | Tipo |
|---|---|
| category | TEXT PK |
| monthly_amount | INTEGER |

### `goals`
| Campo | Tipo |
|---|---|
| id | INTEGER PK |
| name | TEXT |
| target_amount | INTEGER |
| target_date | TEXT (nullable) |
| status | TEXT (`activo` / `completado` / `pausado`) |
| created_at | TEXT |

`current_amount` no se almacena: se calcula con `SELECT SUM(amount) FROM transactions WHERE goal_id = ?`.

### `gas_prices`
| Campo | Tipo |
|---|---|
| id | INTEGER PK |
| date | TEXT |
| price_per_gallon | INTEGER |
| source | TEXT (`manual` / `scraping`) |

### `config`
| Campo | Tipo |
|---|---|
| key | TEXT PK |
| value | TEXT |

---

## 8. Pantallas

4 vistas, navegación por tabs en la parte superior. Sin sidebar.

| Tab | Contenido |
|---|---|
| **Resumen** | Selector de período. KPIs: ingresos, gastos, saldo. Barras vs meta por categoría. Indicador vs mes anterior. Últimas 5 transacciones. Objetivos activos con progreso |
| **Registrar** | Formulario único: tipo, categoría, monto, fecha, nota, checkbox extraordinario, dropdown opcional de objetivo asociado |
| **Historial** | Tabla filtrable. Editar/eliminar inline. Botón exportar CSV |
| **Configuración** | Metas por categoría, parámetros de moto, gasolina (precio + histórico + scraping toggle), objetivos (CRUD), token Turso |

---

## 9. System tray y segundo plano

Al iniciar el sistema, la app arranca minimizada al tray (vía `tauri-plugin-autostart`). Menú contextual del ícono:

```
[Abrir Finanzas]
[Registrar gasto rápido]
─────────────
[Salir]
```

Click izquierdo en ícono → abre/oculta ventana. Click X de la ventana → minimiza al tray (no cierra). Salir explícito requiere usar el menú del tray.

---

## 10. Notificaciones

| Evento | Disparador |
|---|---|
| Excedió meta de categoría en el mes actual | Al guardar una transacción que cruza el umbral |
| Objetivo alcanzado | Al guardar una transacción que completa un objetivo |
| Precio gasolina cambió >5% | Al actualizar precio (manual o scraping) |

Implementación: `tauri-plugin-notification` invocado desde el backend Rust después de la operación de DB. Sin tabla de log.

---

## 11. Scraping de gasolina

- Una fuente (a definir en fase Desarrollo).
- Trigger: manual desde Configuración, o automático al iniciar la app si pasaron >24h del último intento exitoso.
- Si falla 2 veces consecutivas: deshabilitar 7 días, mostrar banner para entrada manual.
- Validación: valor scrapeado debe estar en rango 10.000–25.000 COP antes de guardarse.

---

## 12. Sincronización entre dispositivos

Turso embedded replica en Rust:

```rust
let db = Builder::new_remote_replica(
    "local.db",
    "libsql://finanzas-{user}.turso.io",
    auth_token,
).build().await?;
```

- Lecturas: instantáneas desde la replica local.
- Escrituras: van a la replica local + se replican a Turso en background.
- `db.sync()` se llama: al iniciar la app, después de cada escritura, y cada N minutos si la ventana está abierta.
- Conflictos: last-write-wins.

---

## 13. Instalación

**Linux (Fedora 44):**
1. Descargar `Finanzas.AppImage` del release.
2. `chmod +x Finanzas.AppImage`.
3. Doble click. Primera vez pide pegar token Turso.
4. Opcional: AppImageLauncher para integrar al menú.

**Windows:**
1. Descargar `Finanzas-Setup.msi`.
2. Doble click → instalador → siguiente.
3. Aparece en menú inicio + tray. Primera vez pide pegar token Turso.

Token Turso se obtiene una sola vez en `turso.tech` y se guarda local. Mismo token en ambas máquinas = sync automático.

---

## 14. Fases del proyecto

### Fase 1 — Entendimiento
**Estado: completada.** Este documento.

Objetivo definido. Alcance acotado. Stack confirmado: Tauri + Rust + Svelte + Turso. Decisiones cosméticas resueltas: tema oscuro fijo, repo privado, nombre `Finanzas`, scraping a investigar después.

### Fase 2 — Diseño

Trabajo previo a escribir código de aplicación. Entregables:

- Diagrama Entidad-Relación en DBML (dbdiagram.io).
- `schema.sql` definitivo con índices y constraints.
- Wireframes de las 4 pantallas (texto estructurado o dibujo simple).
- Contrato de comandos Rust ↔ Svelte: lista de funciones que el frontend invocará vía `invoke()` con sus parámetros y tipos de retorno.
- Setup de cuenta Turso + creación de la base de datos remota.
- Setup de repo privado en GitHub.
- Decisión de fuente de scraping de gasolina.

### Fase 3 — Desarrollo

Iteración por hitos pequeños y funcionales. Cada hito = un commit/release interno.

| Hito | Entregable |
|---|---|
| D-1 | Setup proyecto Tauri + Svelte. Conexión Turso. Schema aplicado. Seed inicial |
| D-2 | CRUD transacciones funcionando. Vista Resumen básica con selector de período |
| D-3 | Barras vs meta. Comparativa mes anterior |
| D-4 | Tab Historial: filtros, editar, eliminar, exportar CSV |
| D-5 | Objetivos: CRUD + progreso + asociación con transactions |
| D-6 | Gasolina: registro manual, histórico, comparativa semanal |
| D-7 | Scraping gasolina (opcional) |
| D-8 | System tray + autoarranque + notificaciones |
| D-9 | Tab Configuración completo |

### Fase 4 — Pruebas y validación

| Tipo | Qué se valida |
|---|---|
| Funcional por módulo | Cada hito Dx se prueba antes de cerrar |
| Sync | Probar con 2 máquinas: escribir en A, ver en B después de sync |
| Multiplataforma | Build limpio en Linux Fedora 44 y Windows 11. Probar instalación desde cero |
| Modo offline | Apagar internet, registrar transacciones, prender internet, verificar sync correcto |
| Dogfooding | Usar la app durante 2 semanas como herramienta principal de gestión financiera; documentar fricciones |

### Fase 5 — Producción y Operación

| Tarea | Descripción |
|---|---|
| Build releases | AppImage Linux + MSI Windows publicados en GitHub Releases del repo privado |
| Documentación de instalación | README en el repo con pasos de instalación y obtención de token Turso |
| Monitoreo de memoria | Medir consumo real en idle. Si >60 MB consistentemente, evaluar refactor a daemon + window |
| Backups | Script manual o botón en Configuración para copiar `.db` a una ubicación segura |
| Iteraciones | Ajustes basados en uso real. Cada cambio significativo entra como ticket en el repo |

---

## 15. Riesgos

| Riesgo | Mitigación |
|---|---|
| Turso cambia política free tier | Plan B: SQLite local + Syncthing del archivo `.db` |
| Scraping falla por cambios de HTML | Manual sigue siendo entrada principal, no es bloqueante |
| Curva de Rust al inicio | Tauri requiere poco Rust al inicio. Frontend Svelte cubre ~70% del trabajo |
| Sync genera conflictos | Last-write-wins; uso de una persona alternando máquinas hace conflictos casi imposibles |
| Tray no funciona igual en distros Linux | Probar en GNOME 46 (Fedora 44). Fallback: app sin tray |
| Memoria en idle excede umbral aceptable | Refactor a daemon + window en fase de Operación |
