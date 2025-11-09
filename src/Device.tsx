import { useCallback, useEffect, useRef, useState } from "react";
import "./Device.css";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

export type DeviceInfo = {
  name: string;
  id: number;
  uuid: string;
  connectionType: "USB" | "Network" | "Unknown";
};

export const Device = ({
  selectedDevice,
  setSelectedDevice,
}: {
  selectedDevice: DeviceInfo | null;
  setSelectedDevice: (device: DeviceInfo | null) => void;
}) => {
  const [devices, setDevices] = useState<DeviceInfo[]>([]);

  const listingDevices = useRef<boolean>(false);

  const selectDevice = useCallback(
    (device: DeviceInfo | null) => {
      setSelectedDevice(device);
      invoke("set_selected_device", { device }).catch((err) => {
        toast.error("Failed to select device" + err);
      });
    },
    [setSelectedDevice]
  );

  const loadDevices = useCallback(async () => {
    if (listingDevices.current) return;
    const promise = new Promise<number>(async (resolve, reject) => {
      listingDevices.current = true;
      try {
        const devices = await invoke<DeviceInfo[]>("list_devices");
        setDevices(devices);
        selectDevice(devices.length > 0 ? devices[0] : null);
        listingDevices.current = false;
        resolve(devices.length);
      } catch (e) {
        setDevices([]);
        selectDevice(null);
        listingDevices.current = false;
        reject(e);
      }
    });

    toast.promise(promise, {
      loading: "Loading devices...",
      success: (count) => {
        if (count === 0) {
          return "No devices found";
        }
        return `Found device${count > 1 ? "s" : ""}`;
      },
      error: "Failed to load devices",
    });
  }, [setDevices, selectDevice]);
  useEffect(() => {
    loadDevices();
  }, [loadDevices]);

  return (
    <>
      <h2>iDevice</h2>
      <div className="credentials-container">
        {devices.length === 0 && <div>No devices found.</div>}
        {devices.map((device) => (
          <div
            key={device.id}
            className={
              "device-card card" +
              (selectedDevice?.id === device.id ? " green" : "")
            }
          >
            {device.name} ({device.connectionType})
            <div className="select-device" onClick={() => selectDevice(device)}>
              Select
            </div>
          </div>
        ))}
        <button onClick={loadDevices}>Refresh</button>
      </div>
    </>
  );
};
