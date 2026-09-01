import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { t } from '../i18n';

interface GitHubRelease {
  tag_name: string;
  name: string;
  html_url: string;
  assets: Array<{
    name: string;
    browser_download_url: string;
    size: number;
  }>;
  published_at: string;
}

export function UpdateChecker() {
  const [updateAvailable, setUpdateAvailable] = useState<GitHubRelease | null>(null);
  const [isChecking, setIsChecking] = useState(false);
  const [isDownloading, setIsDownloading] = useState(false);
  const [appVersion, setAppVersion] = useState<string>("");

  // Version comes from the BACKEND (Cargo package version — single source of
  // truth). Never a hardcoded frontend string.
  useEffect(() => {
    invoke<string>('app_version').then(setAppVersion).catch(() => setAppVersion(""));
  }, []);

  const checkForUpdates = async () => {
    if (!appVersion) return;
    setIsChecking(true);
    try {
      const release = await invoke('check_for_updates', {
        currentVersion: appVersion
      }) as GitHubRelease | null;
      setUpdateAvailable(release);
    } catch (error) {
      console.error('Update check failed:', error);
    } finally {
      setIsChecking(false);
    }
  };

  const downloadUpdate = async () => {
    if (!updateAvailable) return;

    const exeAsset = updateAvailable.assets.find(a =>
      a.name.includes('Windows-x64.exe') && !a.name.includes('Setup')
    );

    if (!exeAsset) {
      alert('Windows executable not found in release');
      return;
    }

    setIsDownloading(true);

    try {
      const tempPath = await invoke('get_temp_path') as string;
      const savePath = `${tempPath}/TreeFrog-Content-Manager-${updateAvailable.tag_name}.exe`;

      await invoke('download_update', {
        url: exeAsset.browser_download_url,
        savePath: savePath,
      });

      await invoke('open_folder', { path: tempPath });

      alert(`Update downloaded to:\n${savePath}\n\nPlease close the current app and run the new version.`);
    } catch (error) {
      console.error('Download failed:', error);
      alert(`Download failed: ${error}`);
    } finally {
      setIsDownloading(false);
    }
  };

  useEffect(() => {
    if (appVersion) checkForUpdates();
  }, [appVersion]);

  return (
    <div style={{ 
      marginTop: '20px', 
      padding: '15px', 
      border: '1px solid var(--border-color)',
      borderRadius: '8px',
      backgroundColor: 'var(--bg-secondary, var(--surface))'
    }}>
      <h3 style={{ color: 'var(--text-primary)', marginBottom: '10px' }}>
        {t.updateCheck}
      </h3>
      
      <button
        onClick={checkForUpdates}
        disabled={isChecking}
        style={{
          padding: '8px 16px',
          backgroundColor: 'var(--accent)',
          color: 'var(--button-text)',
          border: 'none',
          borderRadius: '4px',
          cursor: isChecking ? 'not-allowed' : 'pointer',
          marginRight: '10px',
        }}
      >
        {isChecking ? t.checking : t.checkForUpdates}
      </button>

      {updateAvailable && (
        <div style={{ marginTop: '15px', padding: '10px', backgroundColor: 'var(--success-bg)', borderRadius: '4px' }}>
          <p style={{ color: 'var(--text-primary)', margin: '0 0 10px 0' }}>
            <strong>{t.newVersionAvailable}</strong><br />
            {updateAvailable.name} ({updateAvailable.tag_name})
          </p>
          <button
            onClick={downloadUpdate}
            disabled={isDownloading}
            style={{
              padding: '8px 16px',
              backgroundColor: 'var(--success)',
              color: 'var(--button-text)',
              border: 'none',
              borderRadius: '4px',
              cursor: isDownloading ? 'not-allowed' : 'pointer',
            }}
          >
            {isDownloading ? t.downloading : t.downloadUpdate}
          </button>
        </div>
      )}

      {!updateAvailable && !isChecking && (
        <p style={{ color: 'var(--text-secondary)', marginTop: '10px' }}>
          {t.upToDate}
        </p>
      )}
    </div>
  );
}
