import logo from "../assets/soundrix-logo.jpg";
import "./theme.css";

function App() {
  return (
    <main className="shell">
      <header className="top">
        <img className="mark" src={logo} alt="" />
        <div>
          <h1>Soundrix</h1>
          <p>Soundboard</p>
        </div>
      </header>
      <section className="board" aria-label="Sound list">
        <p>No sounds yet.</p>
      </section>
    </main>
  );
}

export default App;
