import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { HardDrive, Smartphone, RefreshCw, CheckCircle, XCircle, Folder, Download } from "lucide-react";
import "./App.css";

interface Device {
  id: string;
  state: string;
  model: string;
}

interface AppInfo {
  package_name: string;
  path: string;
  version: string;
  is_system: boolean;
}

interface BackupProgress {
  package_name: string;
  status: string;
  percentage: number;
}

function App() {
  const [devices, setDevices] = useState<Device[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<string | null>(null);
  const [apps, setApps] = useState<AppInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [hasRoot, setHasRoot] = useState(false);
  const [search, setSearch] = useState("");
  const [selectedApps, setSelectedApps] = useState<Set<string>>(new Set());

  const [outputDir, setOutputDir] = useState("C:\\AndroidBackups");
  const [isProcessing, setIsProcessing] = useState(false);
  const [progress, setProgress] = useState<BackupProgress | null>(null);

  const refreshDevices = async () => {
    try {
      const devList = await invoke<Device[]>("get_devices");
      setDevices(devList);
      if (devList.length > 0 && !selectedDevice) {
        setSelectedDevice(devList[0].id);
      } else if (devList.length === 0) {
        setSelectedDevice(null);
        setApps([]);
      }
    } catch (error) {
      console.error("Failed to refresh devices", error);
    }
  };

  const loadAppsAndRootInfo = async () => {
    if (!selectedDevice) return;
    setLoading(true);
    setSelectedApps(new Set());
    try {
      const rootAvailable = await invoke<boolean>("check_root", { deviceId: selectedDevice });
      setHasRoot(rootAvailable);

      const appList = await invoke<AppInfo[]>("get_apps", { deviceId: selectedDevice });
      setApps(appList);
    } catch (error) {
      console.error("Failed to load apps", error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refreshDevices();
    const interval = setInterval(refreshDevices, 5000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    loadAppsAndRootInfo();
  }, [selectedDevice]);

  useEffect(() => {
    const unlisten = listen<BackupProgress>("backup-progress", (event) => {
      setProgress(event.payload);
      if (event.payload.percentage === 100) {
        setTimeout(() => setIsProcessing(false), 1000);
      }
    });

    return () => {
      unlisten.then(f => f());
    };
  }, []);

  const toggleAppSelection = (pkg: string) => {
    const newSet = new Set(selectedApps);
    if (newSet.has(pkg)) {
      newSet.delete(pkg);
    } else {
      newSet.add(pkg);
    }
    setSelectedApps(newSet);
  };

  const startOperation = async (operation: "backup" | "restore") => {
    if (!selectedDevice || selectedApps.size === 0) return;

    setIsProcessing(true);
    setProgress(null);

    try {
      const command = operation === "backup" ? "start_backup" : "start_restore";
      await invoke(command, {
        request: {
          device_id: selectedDevice,
          apps: Array.from(selectedApps),
          output_dir: outputDir,
          backup_apk: true,
          backup_data: hasRoot
        }
      });
    } catch (e) {
      console.error(`${operation} failed`, e);
      setIsProcessing(false);
    }
  };

  const filteredApps = apps.filter(app =>
    app.package_name.toLowerCase().includes(search.toLowerCase()) &&
    !app.is_system
  );

  return (
    <div className="flex h-screen bg-gray-50 dark:bg-gray-900 text-gray-900 dark:text-gray-100 font-sans">
      {/* Sidebar */}
      <div className="w-64 border-r border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-800 flex flex-col">
        <div className="p-4 border-b border-gray-200 dark:border-gray-700">
          <h1 className="text-xl font-bold flex items-center gap-2">
            <HardDrive className="w-6 h-6 text-blue-500" />
            DroidBackup
          </h1>
        </div>

        <div className="p-4 flex-1 overflow-y-auto">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-semibold text-gray-500 uppercase">Devices</h2>
            <button onClick={refreshDevices} className="text-blue-500 hover:text-blue-600">
              <RefreshCw className="w-4 h-4" />
            </button>
          </div>

          {devices.length === 0 ? (
            <div className="text-sm text-gray-500 italic">No devices found... Connect via USB.</div>
          ) : (
            <ul className="space-y-2">
              {devices.map(dev => (
                <li
                  key={dev.id}
                  onClick={() => setSelectedDevice(dev.id)}
                  className={`p-3 rounded-lg cursor-pointer flex flex-col gap-1 transition-colors ${selectedDevice === dev.id ? 'bg-blue-100 dark:bg-blue-900 border-blue-500' : 'bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600'}`}
                >
                  <div className="flex items-center gap-2 font-medium">
                    <Smartphone className="w-4 h-4" />
                    {dev.model}
                  </div>
                  <div className="text-xs text-gray-500 flex justify-between">
                    <span>{dev.id}</span>
                    <span className={dev.state === 'device' ? 'text-green-500' : 'text-yellow-500'}>{dev.state}</span>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 flex flex-col overflow-hidden relative">
        {selectedDevice ? (
          <>
            <div className="p-6 border-b border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-800 flex justify-between items-center">
              <div>
                <h2 className="text-2xl font-semibold">Select Apps</h2>
                <div className="flex items-center gap-4 mt-2 text-sm text-gray-500">
                  <span className="flex items-center gap-1">
                    Root Status:
                    {hasRoot ? <CheckCircle className="w-4 h-4 text-green-500"/> : <XCircle className="w-4 h-4 text-red-500"/>}
                  </span>
                  <span>Apps: {filteredApps.length}</span>
                  <span>Selected: {selectedApps.size}</span>
                </div>
              </div>
              <input
                type="text"
                placeholder="Search package..."
                className="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-gray-50 dark:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
            </div>

            <div className="flex-1 overflow-y-auto p-6">
              {loading ? (
                <div className="flex items-center justify-center h-full text-gray-500">
                  <RefreshCw className="w-8 h-8 animate-spin" />
                </div>
              ) : (
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                  {filteredApps.map(app => (
                    <div
                      key={app.package_name}
                      onClick={() => toggleAppSelection(app.package_name)}
                      className={`p-4 bg-white dark:bg-gray-800 border rounded-lg shadow-sm hover:shadow-md transition-shadow cursor-pointer ${selectedApps.has(app.package_name) ? 'border-blue-500 ring-1 ring-blue-500' : 'border-gray-200 dark:border-gray-700'}`}
                    >
                      <div className="flex items-start justify-between">
                        <div className="truncate pr-2 font-medium text-sm" title={app.package_name}>
                          {app.package_name}
                        </div>
                        <input
                          type="checkbox"
                          checked={selectedApps.has(app.package_name)}
                          readOnly
                          className="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500"
                        />
                      </div>
                      <div className="mt-2 text-xs text-gray-500 truncate" title={app.path}>
                        {app.path}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>

            <div className="p-4 border-t border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-800 flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Folder className="w-5 h-5 text-gray-500" />
                <input
                  type="text"
                  value={outputDir}
                  onChange={(e) => setOutputDir(e.target.value)}
                  className="w-64 px-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded focus:outline-none focus:ring-2 focus:ring-blue-500 bg-gray-50 dark:bg-gray-700"
                />
              </div>
              <div className="flex gap-2">
                <button
                  onClick={() => startOperation("restore")}
                  disabled={selectedApps.size === 0 || isProcessing}
                  className="flex items-center gap-2 px-6 py-2 bg-green-600 hover:bg-green-700 disabled:bg-gray-400 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-colors"
                >
                  <Download className="w-4 h-4" />
                  Restore Selected
                </button>
                <button
                  onClick={() => startOperation("backup")}
                  disabled={selectedApps.size === 0 || isProcessing}
                  className="flex items-center gap-2 px-6 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed text-white rounded-lg font-medium transition-colors"
                >
                  <HardDrive className="w-4 h-4" />
                  Backup Selected
                </button>
              </div>
            </div>

            {/* Overlay Progress Indicator */}
            {isProcessing && progress && (
              <div className="absolute inset-0 bg-white/80 dark:bg-gray-900/80 backdrop-blur-sm flex items-center justify-center">
                <div className="bg-white dark:bg-gray-800 p-8 rounded-xl shadow-2xl max-w-md w-full border border-gray-200 dark:border-gray-700">
                  <h3 className="text-lg font-bold mb-4">Operation in Progress</h3>
                  <div className="mb-2 text-sm text-gray-600 dark:text-gray-400 font-medium truncate">
                    {progress.package_name}
                  </div>
                  <div className="flex justify-between text-xs text-gray-500 mb-2">
                    <span>{progress.status}</span>
                    <span>{progress.percentage}%</span>
                  </div>
                  <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2.5">
                    <div className="bg-blue-600 h-2.5 rounded-full transition-all duration-300" style={{ width: `${progress.percentage}%` }}></div>
                  </div>
                </div>
              </div>
            )}
          </>
        ) : (
          <div className="flex items-center justify-center h-full text-gray-400 flex-col gap-4">
            <Smartphone className="w-16 h-16 opacity-50" />
            <p className="text-lg">Please connect a device to continue.</p>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
