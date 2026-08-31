import { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { t } from '../i18n';
import EmptyState from "./EmptyState";

interface MusicTrack {
  path: string;
  filename: string;
  size: number;
  folder: string;
}

interface MusicPlaylist {
  name: string;
  path: string;
  tracks: MusicTrack[];
  total_size: number;
}

interface MusicScanResult {
  standalone_tracks: MusicTrack[];
  playlists: MusicPlaylist[];
  total_tracks: number;
  total_playlists: number;
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
  summary: { new: number; unchanged: number; duplicate_content: number; conflicts: number; deletions: number; manual_review?: number; unsupported_archive?: number };
  entries: PlanEntry[];
  warnings: string[];
};

export default function MusicPanel({
  globalSdPath,
  onSourceChange,
  onPlanChange,
  onNext,
  visible
}: {
  globalSdPath: string;
  onSourceChange?: (v: string) => void;
  onPlanChange?: (plan: Plan | null) => void;
  onNext?: () => void;
  visible?: boolean;
}) {
  void globalSdPath;
  void onNext;
  void visible;
  const [musicSource, setMusicSource] = useState('');
  const [scanResult, setScanResult] = useState<MusicScanResult | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedTracks, setSelectedTracks] = useState<Set<string>>(new Set());
  const [expandedPlaylists, setExpandedPlaylists] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(false);
  void loading;
  void setLoading;
  const [error, setError] = useState("");

  const handleBrowse = async () => {
    try {
      const selected = await open({ directory: true });
      if (selected) {
        setMusicSource(selected as string);
        onSourceChange?.(selected as string);
        const result = await invoke('scan_music_structured', { path: selected }) as MusicScanResult;
        setScanResult(result);
        const allTracks = new Set<string>();
        result.standalone_tracks.forEach(t => allTracks.add(t.path));
        result.playlists.forEach(p => p.tracks.forEach(t => allTracks.add(t.path)));
        setSelectedTracks(allTracks);
        setExpandedPlaylists(new Set(result.playlists.map(p => p.name)));
      }
    } catch (e) {
      console.error('Music scan failed:', e);
      setError(String(e));
    }
  };

  useEffect(() => {
    if (!scanResult) {
      onPlanChange?.(null);
      return;
    }
    const selectedStandalone = scanResult.standalone_tracks.filter(t => selectedTracks.has(t.path));
    const selectedPlaylistTracks = scanResult.playlists.flatMap(p => p.tracks.filter(t => selectedTracks.has(t.path)));
    const allSelected = [...selectedStandalone, ...selectedPlaylistTracks];
    if (allSelected.length === 0) {
      onPlanChange?.(null);
      return;
    }
    const entries = allSelected.map(track => ({
      source: track.path,
      destination: track.folder ? `roms/music/${track.folder}/${track.filename}` : `roms/music/${track.filename}`,
      action: 'copy',
      reason: track.folder ? `Music playlist: ${track.folder.split(/[\\/]/)[0]}` : 'Standalone music track',
      content_type: 'music',
      size: track.size,
    }));
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
  }, [scanResult, selectedTracks, onPlanChange]);

  const toggleTrack = (trackPath: string) => {
    setSelectedTracks(prev => {
      const next = new Set(prev);
      if (next.has(trackPath)) next.delete(trackPath);
      else next.add(trackPath);
      return next;
    });
  };

  const togglePlaylist = (playlist: MusicPlaylist, select: boolean) => {
    setSelectedTracks(prev => {
      const next = new Set(prev);
      playlist.tracks.forEach(track => {
        if (select) next.add(track.path);
        else next.delete(track.path);
      });
      return next;
    });
  };

  const togglePlaylistExpanded = (playlistName: string) => {
    setExpandedPlaylists(prev => {
      const next = new Set(prev);
      if (next.has(playlistName)) next.delete(playlistName);
      else next.add(playlistName);
      return next;
    });
  };

  const formatSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const filteredStandalone = useMemo(() => {
    if (!scanResult) return [];
    const q = searchQuery.toLowerCase();
    if (!q) return scanResult.standalone_tracks;
    return scanResult.standalone_tracks.filter(t => t.filename.toLowerCase().includes(q));
  }, [scanResult, searchQuery]);

  const filteredPlaylists = useMemo(() => {
    if (!scanResult) return [];
    const q = searchQuery.toLowerCase();
    if (!q) return scanResult.playlists;
    return scanResult.playlists
      .map(playlist => ({
        ...playlist,
        tracks: playlist.tracks.filter(t => t.filename.toLowerCase().includes(q) || playlist.name.toLowerCase().includes(q)),
      }))
      .filter(playlist => playlist.tracks.length > 0 || playlist.name.toLowerCase().includes(q));
  }, [scanResult, searchQuery]);

  return (
    <div className="card">
      <h3>Music — Playlists (TreeFrogUI)</h3>
      <p style={{ fontSize: 12, color: "var(--text-muted)" }}>
        {t.musicHelp || "Standalone files go to roms/music/. Folders act as playlists in TreeFrogUI."}
      </p>

      <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 12 }}>
        <label style={{ fontSize: 13, fontWeight: 600 }}>Music source folder</label>
        <div className="row" style={{ alignItems: "stretch" }}>
          <div style={{ flex: 1, padding: "8px 10px", border: "1px solid var(--border)", borderRadius: 6, background: "var(--input)", color: musicSource ? "var(--text)" : "var(--text-muted)", fontSize: 13, minHeight: 36, display: "flex", alignItems: "center" }}>
            {musicSource || "No folder selected — e.g., D:\\Music"}
          </div>
          <button onClick={handleBrowse}>Browse</button>
        </div>
      </div>

      <div style={{ marginBottom: '10px' }}>
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder={t.searchPlaceholder || "Search file..."}
          style={{
            width: '100%',
            padding: '8px 12px',
            borderRadius: '4px',
            border: '1px solid var(--border-color)',
            backgroundColor: 'var(--input-bg, transparent)',
            color: 'var(--text-primary)',
          }}
        />
      </div>

      {error && <div className="status-error" style={{ fontSize: 12, marginTop: 8 }}>{error}</div>}
      {loading && <EmptyState kind="loading" title="Scanning Music…" description="Scanning music files..." />}

      {scanResult && (
        <>
          <div style={{ fontSize: 12, color: "var(--text-muted)", marginBottom: '10px' }}>
            {scanResult.total_tracks} tracks, {scanResult.total_playlists} playlists
          </div>

          {filteredStandalone.length > 0 && (
            <div style={{ marginTop: '20px' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '10px' }}>
                <input
                  type="checkbox"
                  checked={filteredStandalone.every(t => selectedTracks.has(t.path))}
                  onChange={(e) => {
                    setSelectedTracks(prev => {
                      const next = new Set(prev);
                      filteredStandalone.forEach(track => {
                        if (e.target.checked) next.add(track.path);
                        else next.delete(track.path);
                      });
                      return next;
                    });
                  }}
                />
                <strong style={{ color: 'var(--text-primary)' }}>
                  🎵 Standalone Tracks ({filteredStandalone.length})
                </strong>
              </div>
              <div style={{ marginLeft: '20px' }}>
                {filteredStandalone.map(track => (
                  <label key={track.path} style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '8px',
                    padding: '6px 8px',
                    borderRadius: '4px',
                    cursor: 'pointer',
                    backgroundColor: selectedTracks.has(track.path) ? 'var(--accent-bg, rgba(25,118,210,0.1))' : 'transparent',
                  }}>
                    <input
                      type="checkbox"
                      checked={selectedTracks.has(track.path)}
                      onChange={() => toggleTrack(track.path)}
                    />
                    <span style={{ color: 'var(--text-primary)', fontSize: '13px' }}>
                      {track.filename}
                    </span>
                    <span style={{ color: 'var(--text-secondary)', fontSize: '12px', marginLeft: 'auto' }}>
                      {formatSize(track.size)}
                    </span>
                  </label>
                ))}
              </div>
            </div>
          )}

          {filteredPlaylists.length > 0 && (
            <div style={{ marginTop: '20px' }}>
              <strong style={{ color: 'var(--text-primary)', display: 'block', marginBottom: '10px' }}>
                📁 Playlists ({filteredPlaylists.length})
              </strong>
              {filteredPlaylists.map(playlist => {
                const isExpanded = expandedPlaylists.has(playlist.name);
                const allSelected = playlist.tracks.every(t => selectedTracks.has(t.path));
                const someSelected = playlist.tracks.some(t => selectedTracks.has(t.path));
                return (
                  <div key={playlist.name} style={{
                    marginBottom: '8px',
                    border: '1px solid var(--border-color)',
                    borderRadius: '6px',
                    overflow: 'hidden',
                  }}>
                    <div style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: '8px',
                      padding: '10px 12px',
                      backgroundColor: 'var(--bg-secondary)',
                      cursor: 'pointer',
                    }}
                    onClick={() => togglePlaylistExpanded(playlist.name)}
                    >
                      <span style={{ color: 'var(--text-secondary)' }}>
                        {isExpanded ? '▼' : '▶'}
                      </span>
                      <input
                        type="checkbox"
                        checked={allSelected}
                        ref={(el) => { if (el) el.indeterminate = someSelected && !allSelected; }}
                        onChange={(e) => {
                          e.stopPropagation();
                          togglePlaylist(playlist, e.target.checked);
                        }}
                        onClick={(e) => e.stopPropagation()}
                      />
                      <strong style={{ color: 'var(--text-primary)', flex: 1 }}>
                        {playlist.name}
                      </strong>
                      <span style={{ color: 'var(--text-secondary)', fontSize: '12px' }}>
                        {playlist.tracks.length} tracks • {formatSize(playlist.total_size)}
                      </span>
                    </div>
                    {isExpanded && (
                      <div style={{ padding: '8px 12px', backgroundColor: 'var(--bg-primary)' }}>
                        {playlist.tracks.map(track => (
                          <label key={track.path} style={{
                            display: 'flex',
                            alignItems: 'center',
                            gap: '8px',
                            padding: '4px 6px',
                            borderRadius: '4px',
                            cursor: 'pointer',
                            backgroundColor: selectedTracks.has(track.path) ? 'var(--accent-bg, rgba(25,118,210,0.1))' : 'transparent',
                          }}>
                            <input
                              type="checkbox"
                              checked={selectedTracks.has(track.path)}
                              onChange={() => toggleTrack(track.path)}
                            />
                            <span style={{ color: 'var(--text-primary)', fontSize: '13px' }}>
                              {track.filename}
                            </span>
                            <span style={{ color: 'var(--text-secondary)', fontSize: '12px', marginLeft: 'auto' }}>
                              {formatSize(track.size)}
                            </span>
                          </label>
                        ))}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}

          {filteredStandalone.length === 0 && filteredPlaylists.length === 0 && (
            <EmptyState kind="empty" title="No Music found" description="No audio files matched search." />
          )}
        </>
      )}

      {!scanResult && !loading && <EmptyState kind="empty" title="No scan yet" description="Select Music source and press Browse. Standalone files and folders will be shown separately." />}
    </div>
  );
}
