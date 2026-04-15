interface Props {
  label: string;
  log: string[];
}

export function InstallProgress({ label, log }: Props) {
  const tail = log.slice(-10);
  return (
    <div className="install-progress">
      <div className="install-progress-label">
        <span className="install-progress-spinner" aria-hidden>⟳</span>
        {label}
      </div>
      {tail.length > 0 && (
        <pre className="install-progress-log">
          {tail.map((line, i) => (
            <div key={i}>{line}</div>
          ))}
        </pre>
      )}
    </div>
  );
}
