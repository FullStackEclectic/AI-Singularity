import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { ArrowRight, Trash2, Power, PowerOff, Plus } from "lucide-react";
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
      <div className="mapping-form" style={{ display: 'flex', gap: 12, alignItems: 'center' }}>
        <input 
          value={source} 
          onChange={e => setSource(e.target.value)} 
          placeholder="拦截来源 (例: gpt-4)" 
          className="form-input" 
          style={{ flex: 1 }}
        />
        <ArrowRight size={16} className="text-muted" />
        <input 
          value={target} 
          onChange={e => setTarget(e.target.value)} 
          placeholder="转发目标 (例: gemini-1.5-pro)" 
          className="form-input" 
          style={{ flex: 1 }}
        />
        <button 
          className="btn btn-primary" 
          onClick={() => addMut.mutate()} 
          disabled={!source || !target || addMut.isPending}
        >
          {addMut.isPending ? "添加中..." : <><Plus size={16}/> 新增</>}
        </button>
      </div>

      <div className="mapping-list" style={{ display: 'flex', flexDirection: 'column', gap: 8, marginTop: 16 }}>
        {mappings.length === 0 && <div className="text-muted" style={{fontSize: 13}}>暂未配置任何拦截映射规则</div>}
        {mappings.map(m => (
          <div key={m.id} className={`protocol-item ${!m.is_active ? "opacity-50" : ""}`}>
            <div className="protocol-meta" style={{ flex: 1 }}>
              <span className="protocol-tag" style={{ minWidth: 100, textAlign: 'center' }}>{m.source_model}</span>
              <ArrowRight size={14} className="text-muted" />
              <span className="protocol-flow text-primary">{m.target_model}</span>
            </div>
            <div className="mapping-actions" style={{ display: 'flex', gap: 8 }}>
               <button 
                 className={`btn-icon ${m.is_active ? "text-success" : "text-muted"}`} 
                 onClick={() => toggleMut.mutate(m)}
               >
                 {m.is_active ? <Power size={16}/> : <PowerOff size={16}/>}
               </button>
               <button className="btn-icon danger" onClick={() => delMut.mutate(m.id)}>
                 <Trash2 size={16}/>
               </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
