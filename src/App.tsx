export default function App() {
  return (
    <main className="min-h-screen w-full bg-[var(--color-paper)] text-[var(--color-ink)] p-4">
      <header className="border-b border-[var(--color-ink)] pb-2 mb-4 flex justify-between items-center">
        <h1 className="text-xl font-bold tracking-tight">SCRAPE // INITIALIZING</h1>
        <span className="text-xs font-mono uppercase bg-[var(--color-ink)] text-[var(--color-paper)] px-2 py-0.5">
          PHASE 1
        </span>
      </header>
      <div className="border border-[var(--color-ink)] p-4 font-mono text-sm">
        <p className="mb-2">SYSTEM READY. DATABASE & CONFIGURATION ARCHITECTURE.</p>
        <p className="text-[var(--color-ink-mute)]">STATUS: ONLINE</p>
      </div>
    </main>
  );
}
