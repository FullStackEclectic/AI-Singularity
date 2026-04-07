import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import "./ModelMappingPanel.css";

interface ModelMapping {
  id: string;
  source_model: string;
  target_model: string;
  is_active: boolean;
}

export default function ModelMappingPanel() {
  const qc = useQueryClient();
  const [source, setSource] = useState("");
  const [target, setTarget] = useState("");

  const { data: mappings = [] } = useQuery<ModelMapping[]>({
    queryKey: ["model-mappings"],
    queryFn: () => invoke("list_model_mappings"),
  });

  const addMut = useMutation({
    mutationFn: () => invoke("upsert_model_mapping", { req: { source_model: source, target_model: target, is_active: true } }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["model-mappings"] });
      setSource("");
      setTarget("");
    }
  });

  const toggleMut = useMutation({
    mutationFn: (m: ModelMapping) => invoke("upsert_model_mapping", { req: { id: m.id, source_model: m.source_model, target_model: m.target_model, is_active: !m.is_active } }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["model-mappings"] }),
  });

  const delMut = useMutation({
    mutationFn: (id: string) => invoke("delete_model_mapping", { id }),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["model-mappings"] }),
  });

  return (
    <div className="model-mapping-panel">
      <div className="mapping-form">
        <input 
          value={source} 
          onChange={e => setSource(e.target.value)} 
          placeholder="拦截目标 (例: gpt-4)" 
          className="cyber-input" 
        />
        <span className="mapping-arrow">⭢</span>
        <input 
          value={target} 
          onChange={e => setTarget(e.target.value)} 
          placeholder="重录目标 (例: gemini-1.5)" 
          className="cyber-input" 
        />
        <button 
          className="cyber-btn cyber-btn-sm" 
          onClick={() => addMut.mutate()} 
          disabled={!source || !target || addMut.isPending}
        >
          {addMut.isPending ? "INJECTING..." : "ADD_RULE"}
        </button>
      </div>

      <div className="mapping-list">
        {mappings.length === 0 && <div className="text-muted" style={{fontSize: 12}}>NO_ACTIVE_RULES_DETECTED</div>}
        {mappings.map(m => (
          <div key={m.id} className={`mapping-item ${!m.is_active ? "disabled" : ""}`}>
            <div className="mapping-info">
              <span className="source-label">{m.source_model}</span>
              <span className="mapping-arrow">⭢</span>
              <span className="target-label" style={{color: 'var(--color-primary)'}}>{m.target_model}</span>
            </div>
            <div className="mapping-actions">
               <button className="cyber-icon-btn" onClick={() => toggleMut.mutate(m)}>
                 {m.is_active ? "ON" : "OFF"}
               </button>
               <button className="cyber-icon-btn danger" onClick={() => delMut.mutate(m.id)}>
                 DEL
               </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
