import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import { Shield, Cpu, Globe, MapPin, Activity, Terminal, AlertTriangle, Fingerprint } from "lucide-react";

// Animation Variants
const containerVariants = {
  hidden: { opacity: 0 },
  visible: {
    opacity: 1,
    transition: { staggerChildren: 0.1, delayChildren: 0.2 },
  },
};

const itemVariants = {
  hidden: { y: 20, opacity: 0 },
  visible: { y: 0, opacity: 1, transition: { type: "spring", stiffness: 100 } },
};

function App() {
  const [auditData, setAuditData] = useState<any>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Artificial delay for smooth initial animation load
    setTimeout(() => {
      invoke("run_audit")
        .then((res) => {
          setAuditData(res);
          setLoading(false);
        })
        .catch((err) => {
          console.error("Audit error:", err);
          setLoading(false);
        });
    }, 800);
  }, []);

  const parseData = (key: string) => {
    if (!auditData || !auditData[key]) return "Unknown";
    return auditData[key].String || auditData[key].Bool || JSON.stringify(auditData[key]);
  };

  return (
    <div style={{ padding: "2rem", maxWidth: "1200px", margin: "0 auto" }}>
      {/* Header Section */}
      <motion.div 
        initial={{ opacity: 0, y: -20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5 }}
        style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "2.5rem" }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "1rem" }}>
          <div style={{ 
            background: "rgba(139, 92, 246, 0.2)", 
            padding: "12px", 
            borderRadius: "14px",
            border: "1px solid rgba(139, 92, 246, 0.4)"
          }}>
            <Shield size={28} color="#a78bfa" />
          </div>
          <div>
            <h1 className="text-gradient" style={{ fontSize: "2rem", lineHeight: 1 }}>Mirage Platform</h1>
            <p style={{ color: "var(--text-muted)", fontSize: "0.9rem", marginTop: "4px" }}>Identity Virtualization Dashboard</p>
          </div>
        </div>
        
        <div className="glass-panel" style={{ padding: "12px 20px", display: "flex", alignItems: "center", gap: "12px" }}>
          <div className="status-badge status-active">
            <div style={{ width: "8px", height: "8px", borderRadius: "50%", background: "currentColor", boxShadow: "0 0 10px currentColor" }}></div>
            Sandbox Active
          </div>
          <span style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>Profile: <strong style={{ color: "var(--text-main)"}}>Default</strong></span>
        </div>
      </motion.div>

      {/* Warning Banner */}
      <motion.div 
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={{ delay: 0.3 }}
        className="glass-panel" 
        style={{ 
          marginBottom: "2.5rem", 
          padding: "1rem 1.5rem", 
          display: "flex", 
          alignItems: "center", 
          gap: "1rem",
          borderColor: "rgba(245, 158, 11, 0.3)",
          background: "linear-gradient(90deg, rgba(245, 158, 11, 0.05) 0%, transparent 100%)"
        }}
      >
        <AlertTriangle color="var(--accent-warning)" size={24} />
        <div>
          <h4 style={{ color: "var(--accent-warning)", marginBottom: "2px" }}>Terminal Isolation Warning</h4>
          <p style={{ fontSize: "0.85rem", color: "var(--text-muted)" }}>
            'New Tab' inside GUI terminals opens on the host system. Always use <strong>tmux</strong> panes or a new <code>mirage shell</code> instance.
          </p>
        </div>
      </motion.div>

      {/* Main Grid */}
      <motion.div 
        variants={containerVariants}
        initial="hidden"
        animate="visible"
        style={{ 
          display: "grid", 
          gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", 
          gap: "1.5rem",
          marginBottom: "2.5rem"
        }}
      >
        {/* Hardware Card */}
        <motion.div variants={itemVariants} className="glass-panel" style={{ padding: "1.5rem" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "1.5rem" }}>
            <Cpu size={20} color="var(--accent-secondary)" />
            <h3>Hardware Identity</h3>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
            <DataRow label="Hostname" value={loading ? "..." : parseData("\"Hostname\"")} />
            <DataRow label="Machine ID" value={loading ? "..." : parseData("\"MachineId\"")} />
            <DataRow label="MAC Address" value={loading ? "..." : parseData("\"Mac\"")} />
          </div>
        </motion.div>

        {/* Network Card */}
        <motion.div variants={itemVariants} className="glass-panel" style={{ padding: "1.5rem" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "1.5rem" }}>
            <Globe size={20} color="var(--accent-success)" />
            <h3>Network Environment</h3>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
            <DataRow label="IPv4 Address" value={loading ? "..." : parseData("\"Ipv4\"")} />
            <DataRow label="WebRTC Leak" value={loading ? "..." : parseData("\"WebRtc\"")} />
            <DataRow label="DNS Servers" value={loading ? "..." : parseData("\"Dns\"")} />
          </div>
        </motion.div>

        {/* Locale Card */}
        <motion.div variants={itemVariants} className="glass-panel" style={{ padding: "1.5rem" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "1.5rem" }}>
            <MapPin size={20} color="var(--accent-warning)" />
            <h3>Location & Time</h3>
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
            <DataRow label="Timezone" value={loading ? "..." : parseData("\"Timezone\"")} />
            <DataRow label="Locale (LANG)" value={loading ? "..." : parseData("\"Locale\"")} />
            <DataRow label="GeoClue (GPS)" value={loading ? "..." : parseData("\"GeoClue\"")} />
          </div>
        </motion.div>
      </motion.div>

      {/* Raw Audit Data */}
      <motion.div 
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.6 }}
        className="glass-panel" 
        style={{ padding: "1.5rem" }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "1rem" }}>
          <Terminal size={20} color="var(--text-muted)" />
          <h3>Raw Audit Engine Output</h3>
        </div>
        
        <div style={{ 
          background: "rgba(0,0,0,0.3)", 
          borderRadius: "8px", 
          padding: "1rem",
          maxHeight: "300px",
          overflowY: "auto",
          border: "1px solid rgba(255,255,255,0.05)"
        }}>
          <AnimatePresence mode="wait">
            {loading ? (
              <motion.div 
                key="loading"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                style={{ display: "flex", alignItems: "center", gap: "10px", color: "var(--text-muted)" }}
              >
                <Activity className="animate-spin" size={16} />
                <span style={{ fontFamily: "monospace", fontSize: "0.85rem" }}>Initializing audit scan...</span>
              </motion.div>
            ) : (
              <motion.pre 
                key="data"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
              >
                {JSON.stringify(auditData, null, 2)}
              </motion.pre>
            )}
          </AnimatePresence>
        </div>
      </motion.div>
    </div>
  );
}

// Reusable component for data rows
function DataRow({ label, value }: { label: string, value: string }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", borderBottom: "1px solid rgba(255,255,255,0.05)", paddingBottom: "8px" }}>
      <span style={{ color: "var(--text-muted)", fontSize: "0.85rem" }}>{label}</span>
      <span style={{ 
        fontFamily: "monospace", 
        fontSize: "0.85rem",
        color: value === "Unknown" ? "var(--text-muted)" : "var(--text-main)",
        textAlign: "right",
        maxWidth: "60%",
        wordBreak: "break-all"
      }}>
        {value}
      </span>
    </div>
  );
}

export default App;
