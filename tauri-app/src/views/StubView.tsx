// Placeholder for views that haven't been ported from the Swift version
// yet. Renders its own header so the titlebar drag strip and sidebar line
// up the same way as the real views.
interface Props {
  title: string;
  blurb: string;
}

export function StubView({ title, blurb }: Props) {
  return (
    <>
      <header className="main-header">
        <div className="title-block">
          <div className="title-row">
            <h2>{title}</h2>
          </div>
        </div>
      </header>
      <div className="main-body">
        <div className="stub">
          <h3>Coming soon</h3>
          <p>{blurb}</p>
        </div>
      </div>
    </>
  );
}
