import { useState } from "react";
import type { SourceKind } from "../../../bindings/SourceKind";
import { api, type DetectResult } from "../../../lib/api";
import { Modal } from "../../ui/Modal";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { Button } from "../../ui/Button";
import { Field } from "../../ui/Field";
import { Rule } from "../../ui/Rule";

interface AddSourceModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSourceCreated: () => void;
}

const KIND_OPTIONS = [
  { value: "greenhouse", label: "GREENHOUSE" },
  { value: "lever", label: "LEVER" },
  { value: "ashby", label: "ASHBY" },
  { value: "jsonld", label: "JSON-LD (SCHEMA.ORG)" },
  { value: "selector", label: "CSS SELECTOR" },
];

export function AddSourceModal({
  isOpen,
  onClose,
  onSourceCreated,
}: AddSourceModalProps) {
  const [url, setUrl] = useState("");
  const [isDetecting, setIsDetecting] = useState(false);
  const [name, setName] = useState("");
  const [kind, setKind] = useState<SourceKind>("greenhouse");
  const [identifier, setIdentifier] = useState("");
  const [selectorItem, setSelectorItem] = useState(".job-row");
  const [selectorTitle, setSelectorTitle] = useState(".title");
  const [selectorLink, setSelectorLink] = useState("a");
  const [selectorLocation, setSelectorLocation] = useState(".location");
  const [isSaving, setIsSaving] = useState(false);

  const handleDetect = async () => {
    if (!url.trim()) return;
    try {
      setIsDetecting(true);
      const res: DetectResult = await api.detectSource(url.trim());
      setName(res.name);
      setKind(res.kind as SourceKind);
      setIdentifier(res.identifier);
    } catch (e) {
      console.error("Detection error:", e);
    } finally {
      setIsDetecting(false);
    }
  };

  const handleSave = async () => {
    if (!name.trim() || !identifier.trim()) return;

    let config: Record<string, unknown> = {};
    if (kind === "selector") {
      config = {
        item: selectorItem,
        title: selectorTitle,
        link: selectorLink,
        location: selectorLocation,
      };
    }

    try {
      setIsSaving(true);
      await api.createSource({
        name: name.trim(),
        kind,
        identifier: identifier.trim(),
        config,
      });
      onSourceCreated();
      onClose();
      // Reset form
      setUrl("");
      setName("");
      setIdentifier("");
    } catch (e) {
      console.error("Failed to create source:", e);
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="ADD NEW CAREER SOURCE">
      <div className="flex flex-col gap-4 font-mono text-xs">
        {/* Step 1: URL Detection */}
        <div className="flex flex-col gap-2">
          <Field label="CAREERS PAGE OR JOB BOARD URL">
            <div className="flex gap-2">
              <Input
                variant="mono"
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder="https://boards.greenhouse.io/stripe or https://company.com/jobs"
                className="flex-1"
              />
              <Button
                variant="default"
                onClick={handleDetect}
                disabled={isDetecting || !url.trim()}
              >
                {isDetecting ? "DETECTING…" : "AUTO-DETECT"}
              </Button>
            </div>
          </Field>
        </div>

        <Rule />

        {/* Step 2: Source Details */}
        <div className="grid grid-cols-2 gap-3">
          <Field label="COMPANY / SOURCE NAME">
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Stripe, OpenAI"
            />
          </Field>

          <Field label="ADAPTER KIND">
            <Select
              value={kind}
              onChange={(e) => setKind(e.target.value as SourceKind)}
              options={KIND_OPTIONS}
            />
          </Field>
        </div>

        <Field
          label="BOARD IDENTIFIER / TARGET URL"
          hint="SLUG FOR GREENHOUSE/LEVER/ASHBY; FULL URL FOR JSON-LD & SELECTOR"
        >
          <Input
            variant="mono"
            value={identifier}
            onChange={(e) => setIdentifier(e.target.value)}
            placeholder="e.g. stripe or https://example.com/careers"
          />
        </Field>

        {/* CSS Selector configuration if kind === selector */}
        {kind === "selector" && (
          <div className="flex flex-col gap-2 p-3 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30">
            <span className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
              CSS SELECTOR CONFIGURATION
            </span>
            <div className="grid grid-cols-2 gap-2">
              <Field label="ITEM SELECTOR">
                <Input
                  variant="mono"
                  value={selectorItem}
                  onChange={(e) => setSelectorItem(e.target.value)}
                  placeholder=".job-row"
                />
              </Field>
              <Field label="TITLE SELECTOR">
                <Input
                  variant="mono"
                  value={selectorTitle}
                  onChange={(e) => setSelectorTitle(e.target.value)}
                  placeholder=".title"
                />
              </Field>
              <Field label="LINK SELECTOR">
                <Input
                  variant="mono"
                  value={selectorLink}
                  onChange={(e) => setSelectorLink(e.target.value)}
                  placeholder="a"
                />
              </Field>
              <Field label="LOCATION SELECTOR">
                <Input
                  variant="mono"
                  value={selectorLocation}
                  onChange={(e) => setSelectorLocation(e.target.value)}
                  placeholder=".location"
                />
              </Field>
            </div>
          </div>
        )}

        <Rule />

        {/* Actions */}
        <div className="flex justify-end gap-2 pt-2">
          <Button variant="ghost" onClick={onClose}>
            CANCEL
          </Button>
          <Button
            variant="inverted"
            onClick={handleSave}
            disabled={isSaving || !name.trim() || !identifier.trim()}
          >
            {isSaving ? "SAVING…" : "SAVE & ACTIVATE SOURCE"}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
