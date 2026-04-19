import { useState } from "react";
import { Check } from "lucide-react";
import type { ChannelOption } from "./addAccountWizardTypes";
import "./AddAccountChannelSelect.css";

type AddAccountChannelSelectProps = {
  value: string;
  options: readonly ChannelOption[];
  onChange: (value: string) => void;
};

export function AddAccountChannelSelect({
  value,
  options,
  onChange,
}: AddAccountChannelSelectProps) {
  const [isOpen, setIsOpen] = useState(false);
  const selected = options.find((option) => option.value === value) || options[0];

  return (
    <div className="wiz-custom-select-container">
      <div
        className={`wiz-custom-select-trigger ${isOpen ? "open" : ""}`}
        onClick={() => setIsOpen(!isOpen)}
      >
        <div className="wiz-custom-select-val">
          <span className="wiz-cs-label">{selected.label}</span>
          <span className="wiz-cs-desc">{selected.desc}</span>
        </div>
        <div className="wiz-cs-arrow" />
      </div>

      {isOpen && (
        <>
          <div className="wiz-cs-overlay" onClick={() => setIsOpen(false)} />
          <div className="wiz-cs-menu">
            {options.map((option) => (
              <div
                key={option.value}
                className={`wiz-cs-option ${option.value === value ? "selected" : ""}`}
                onClick={() => {
                  onChange(option.value);
                  setIsOpen(false);
                }}
              >
                <div className="wiz-cs-opt-label">{option.label}</div>
                <div className="wiz-cs-opt-desc">{option.desc}</div>
                {option.value === value && <Check size={14} className="wiz-cs-check" />}
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
