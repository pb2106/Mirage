import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";

function App() {
  const [auditData, setAuditData] = useState<any>(null);

  useEffect(() => {
    // In Phase 1, we just call the audit backend
    invoke("run_audit")
      .then((res) => setAuditData(res))
      .catch((err) => console.error("Audit error:", err));
  }, []);

  return (
    <div style={{ padding: "2rem", fontFamily: "sans-serif" }}>
      <h1>Mirage Dashboard - Phase 1</h1>
      <div style={{ marginBottom: "2rem" }}>
        <h2>Active Session: Default</h2>
        <p style={{ color: "#d97706", fontWeight: "bold", marginTop: "0.5rem" }}>
          ⚠ Terminal 'New Tab' opens outside this profile's sandbox. Use tmux panes or a new Mirage Shell window.
        </p>
      </div>
      <h2>Audit Engine Identity Graph</h2>
      {auditData ? (
        <pre style={{ background: "#f0f0f0", padding: "1rem" }}>
          {JSON.stringify(auditData, null, 2)}
        </pre>
      ) : (
        <p>Loading audit data...</p>
      )}
    </div>
  );
}

export default App;
