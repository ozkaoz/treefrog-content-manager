import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { dialogService } from '../services/dialog';
import { t } from '../i18n';
import SdStatusBar from './SdStatusBar';

interface BiosEntry {
  id: string;
  system_name: string;
  filenames: string[];
  pattern?: string;
  destination: string;
  required: boolean;
  description: string;
  sha256?: string;
  md5?: string;
  expected_size?: number;
}

interface BiosState {
  selected: boolean;
  found_path: string | null;
  valid: boolean;
  reason?: string;
}

type PlanEntry = {
  source: string;
  destination: string;
  action: string;
  reason: string;
  content_type?: string;
  size?: number;
};

type Plan = {
  summary: { new: number; changed: number; unchanged: number; duplicate_content: number; conflicts: number; deletions: number; manual_review?: number; unsupported_archive?: number };
  entries: PlanEntry[];
  warnings: string[];
};

export default function BiosManager({
  globalSdPath,
  onSourceChange,
  onPlanChange,
  onBack,
  onSyncToSd,
  sdRefreshSignal = 0,
  onNext,
  visible
}: {
  globalSdPath: string;
  onSourceChange?: (v: string) => void;
  onPlanChange?: (plan: Plan | null) => void;
  onBack?: () => void;
  onSyncToSd?: () => void;
  sdRefreshSignal?: number;
  onNext?: () => void;
  visible?: boolean;
}) {
  void visible;
  const [catalog, setCatalog] = useState<BiosEntry[]>([]);
  const [biosState, setBiosState] = useState<Record<string, BiosState>>({});
  const [error, setError] = useState("");

  useEffect(() => {
    invoke('get_bios_catalog').then((c) => {
      const cat = c as BiosEntry[];
      setCatalog(cat);
      const initial: Record<string, BiosState> = {};
      cat.forEach(b => {
        initial[b.id] = { selected: false, found_path: null, valid: false };
      });
      setBiosState(initial);
    }).catch(e => setError(String(e)));
  }, []);

  useEffect(() => {
    if (!visible || catalog.length === 0) return;
    // No auto-scan, user browses per BIOS
  }, [visible, catalog]);

  const handleBrowse = async (biosId: string) => {
    const bios = catalog.find(b => b.id === biosId);
    if (!bios) return;
    try {
      const selected = await dialogService.pickFile({
        title: 'Select BIOS file',
        filters: [{ name: 'BIOS files', extensions: ['bin', 'rom', 'zip', 'img', 'pk3'] }],
      });
      if (selected) {
        const validation = await invoke('validate_bios_file', {
          path: selected,
          biosId: biosId,
        }) as { valid: boolean; reason: string };
        setBiosState(prev => ({
          ...prev,
          [biosId]: {
            selected: validation.valid,
            found_path: selected,
            valid: validation.valid,
            reason: validation.reason,
          },
        }));
        if (onSourceChange) onSourceChange(selected);
      }
    } catch (e) {
      console.error(e);
      setError(String(e));
    }
  };

  const toggleSelection = (biosId: string) => {
    setBiosState(prev => ({
      ...prev,
      [biosId]: { ...prev[biosId], selected: !prev[biosId].selected },
    }));
  };

  useEffect(() => {
    if (catalog.length === 0) {
      onPlanChange?.(null);
      return;
    }
    const entries: PlanEntry[] = Object.entries(biosState)
      .filter(([_, state]) => state.selected && state.found_path && state.valid)
      .map(([biosId, state]) => {
        const bios = catalog.find(b => b.id === biosId)!;
        const filename = state.found_path!.split(/[\\/]/).pop()!;
        return {
          source: state.found_path!,
          destination: `${bios.destination}/${filename}`,
          action: 'copy',
          reason: 'BIOS file selected by user',
          content_type: 'bios',
          size: 0,
        };
      });
    if (entries.length === 0) {
      onPlanChange?.(null);
      return;
    }
    const summary = {
      new: entries.length,
      changed: 0,
      unchanged: 0,
      duplicate_content: 0,
      conflicts: 0,
      deletions: 0,
      manual_review: 0,
      unsupported_archive: 0,
    };
    onPlanChange?.({ entries, summary, warnings: [] });
  }, [biosState, catalog, onPlanChange]);

  // Search removed: the user selects each BIOS file directly from the catalog.
  const visibleCatalog = catalog;

  return (
    <div className="card">
      <h3>{t.biosManagement || "BIOS Management"}</h3>
      <SdStatusBar sdPath={globalSdPath} refreshSignal={sdRefreshSignal} />
      <p style={{ color: 'var(--text-secondary)', fontSize: '12px', marginBottom: '12px' }}>
        {t.biosHelp || "Select BIOS files required by TreeFrogUI. All files are copied to cubegm/bios/ on the SD card."}
      </p>

      <div className="panel-actions">
        <button className="panel-btn back" onClick={() => onBack?.()}>
          ← Back
        </button>
        <span className="flex-fill" />
        <button className="panel-btn skip" onClick={() => onNext?.()}>
          Skip → LGPT
        </button>
        <button className="panel-btn continue" onClick={() => onNext?.()} disabled={Object.values(biosState).filter(s => s.selected && s.valid).length === 0}>
          Continue to LGPT →
        </button>
        <button className="panel-btn sync" onClick={() => onSyncToSd?.()} disabled={Object.values(biosState).filter(s => s.selected && s.valid).length === 0}>
          Sync to SD →
        </button>
      </div>

      <div style={{
        padding: '12px 16px',
        marginBottom: '16px',
        backgroundColor: 'var(--info-bg, rgba(25, 118, 210, 0.1))',
        border: '1px solid var(--info, #1976d2)',
        borderRadius: '6px',
        color: 'var(--text-primary)',
        fontSize: '13px',
      }}>
        <strong>ℹ️ Note:</strong> Wolfenstein 3D engine (<code>ecwolf.pk3</code>) and DOS boot image (<code>x86BOOT.img</code>) are already included in TreeFrogUI and do not need to be installed.
      </div>

      {error && <div className="status-error" style={{ fontSize: 12, marginTop: 8 }}>{error}</div>}

      <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
        {visibleCatalog.map(bios => {
          const state = biosState[bios.id];
          const statusColor = !state?.found_path
            ? 'var(--text-secondary)'
            : state.valid
            ? 'var(--success, #4CAF50)'
            : 'var(--danger, #d32f2f)';
          const statusIcon = !state?.found_path ? '⚪' : state.valid ? '✅' : '⚠️';

          return (
            <div key={bios.id} style={{
              border: '1px solid var(--border-color)',
              borderRadius: '8px',
              padding: '14px',
              backgroundColor: bios.required ? 'var(--bg-secondary, var(--surface))' : 'transparent',
              borderLeft: bios.required ? '4px solid var(--accent)' : '4px solid var(--border-color)',
            }}>
              <div style={{ display: 'flex', alignItems: 'flex-start', gap: '12px' }}>
                <input
                  type="checkbox"
                  checked={state?.selected || false}
                  disabled={!state?.found_path || !state?.valid}
                  onChange={() => toggleSelection(bios.id)}
                  style={{ marginTop: '4px', transform: 'scale(1.2)' }}
                />
                <div style={{ flex: 1 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '4px' }}>
                    <strong style={{ color: 'var(--text-primary)' }}>
                      {statusIcon} {bios.system_name}
                    </strong>
                    {bios.required && (
                      <span style={{
                        fontSize: '11px', padding: '2px 6px',
                        backgroundColor: 'var(--accent)', color: 'var(--button-text)',
                        borderRadius: '4px', fontWeight: 'bold',
                      }}>
                        REQUIRED
                      </span>
                    )}
                    {!bios.required && (
                      <span style={{
                        fontSize: '11px', padding: '2px 6px',
                        backgroundColor: 'var(--border-color)', color: 'var(--text-secondary)',
                        borderRadius: '4px',
                      }}>
                        OPTIONAL
                      </span>
                    )}
                  </div>
                  <div style={{ fontSize: '13px', color: 'var(--text-secondary)', marginBottom: '8px' }}>
                    {bios.description}
                  </div>
                  <div style={{ fontSize: '12px', color: statusColor, fontFamily: 'monospace', marginBottom: '8px' }}>
                    Expected: {bios.pattern || bios.filenames.join(' OR ')}
                    {state?.found_path && (
                      <div style={{ marginTop: '4px', color: state.valid ? 'var(--success)' : 'var(--danger)' }}>
                        Selected: {state.found_path.split(/[\\/]/).pop()} — {state.reason || (state.valid ? 'File valid' : 'Invalid')}
                      </div>
                    )}
                  </div>
                  <button
                    onClick={() => handleBrowse(bios.id)}
                    style={{
                      padding: '6px 14px', fontSize: '13px',
                      backgroundColor: 'var(--accent)', color: 'var(--button-text)',
                      border: 'none', borderRadius: '4px', cursor: 'pointer',
                    }}
                  >
                    {t.browseBios || "Browse..."}
                  </button>
                </div>
              </div>
            </div>
          );
        })}
        {visibleCatalog.length === 0 && (
          <div style={{ textAlign: 'center', padding: '20px', color: 'var(--text-secondary)' }}>No BIOS entries.</div>
        )}
      </div>

      <div style={{
        padding: '10px',
        backgroundColor: Object.values(biosState).filter(s => s.selected && s.valid).length > 0 ? 'var(--success-bg)' : 'var(--bg-secondary, var(--surface))',
        borderRadius: '6px',
        marginTop: '15px',
      }}>
        <strong>{Object.values(biosState).filter(s => s.selected && s.valid).length} BIOS file(s) selected for sync</strong>
        {Object.values(biosState).filter(s => s.selected && s.valid).length > 0 && (
          <ul style={{ margin: '10px 0', paddingLeft: '20px' }}>
            {Object.entries(biosState).filter(([_, s]) => s.selected && s.valid).map(([biosId, state]) => {
              const bios = catalog.find(b => b.id === biosId);
              const filename = state.found_path?.split(/[\\/]/).pop() || biosId;
              return (
                <li key={biosId} style={{ fontSize: '13px' }}>
                  {bios?.destination}/{filename}
                </li>
              );
            })}
          </ul>
        )}
      </div>

      <p style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 8 }}>All BIOS files will be copied to <code>cubegm/bios/</code> (VICE JiffyDOS to <code>cubegm/bios/vice/</code>) as per TreeFrogUI docs. Required BIOS show blue left border.</p>
    </div>
  );
}
