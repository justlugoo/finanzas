<script lang="ts">
  // Fecha tipeada con máscara DD/MM/AAAA — sin calendario emergente de
  // ningún tipo, así que no hay ventana que "esconder" ni estilo ajeno al
  // de la app. Valida día 01-31, mes 01-12, y no permite fechas futuras.
  let { value = $bindable("") }: { value: string } = $props();

  function todayISO(): string {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }

  function daysIn(year: number, month: number): number {
    return new Date(year, month, 0).getDate();
  }

  function isoToDisplay(iso: string): string {
    const [y, m, d] = (iso || "").split("-");
    if (!y || !m || !d) return "";
    return `${d}/${m}/${y}`;
  }

  let display   = $state(isoToDisplay(value || todayISO()));
  let lastValue = value;

  // Si `value` cambia desde afuera (ej. el formulario se limpia tras
  // guardar), reflejarlo en el campo.
  $effect(() => {
    if (value !== lastValue) {
      lastValue = value;
      display = isoToDisplay(value);
    }
  });

  function handleInput(e: Event & { currentTarget: HTMLInputElement }) {
    const digits = e.currentTarget.value.replace(/\D/g, "").slice(0, 8);

    let dd = digits.slice(0, 2);
    let mm = digits.slice(2, 4);
    const yyyy = digits.slice(4, 8);

    // Clamp progresivo apenas hay 2 dígitos — no deja escribir un día
    // mayor a 31 ni un mes mayor a 12.
    if (dd.length === 2) dd = String(Math.min(31, Math.max(1, parseInt(dd, 10) || 1))).padStart(2, "0");
    if (mm.length === 2) mm = String(Math.min(12, Math.max(1, parseInt(mm, 10) || 1))).padStart(2, "0");

    let out = dd;
    if (digits.length > 2) out += "/" + mm;
    if (digits.length > 4) out += "/" + yyyy;
    display = out;

    if (digits.length === 8) {
      const y = parseInt(yyyy, 10);
      const m = parseInt(mm, 10);
      const d = Math.min(parseInt(dd, 10), daysIn(y, m));
      let iso = `${y}-${String(m).padStart(2, "0")}-${String(d).padStart(2, "0")}`;

      const today = todayISO();
      if (iso > today) iso = today; // no se pueden registrar fechas futuras

      value     = iso;
      lastValue = iso;
      display   = isoToDisplay(iso);
    }
  }
</script>

<input
  class="dp-input"
  type="text"
  inputmode="numeric"
  placeholder="DD/MM/AAAA"
  maxlength="10"
  value={display}
  oninput={handleInput}
/>

<style>
  .dp-input {
    -webkit-appearance: none;
    appearance: none;
    background-color: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 0.85rem;
    padding: 0.5rem 0.65rem;
    outline: none;
    width: 100%;
    transition: border-color 0.15s;
  }
  .dp-input:focus { border-color: var(--accent); }
  .dp-input::placeholder { color: var(--text-muted); }
</style>
