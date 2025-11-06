use std::sync::Mutex;

use idevice::{
    lockdown::LockdownClient,
    provider::UsbmuxdProvider,
    usbmuxd::{UsbmuxdAddr, UsbmuxdConnection},
    IdeviceService,
};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Deserialize, Serialize, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub id: u32,
    pub uuid: String,
}

pub type DeviceInfoMutex = Mutex<Option<DeviceInfo>>;

#[tauri::command]
pub async fn list_devices() -> Result<Vec<DeviceInfo>, String> {
    let usbmuxd = UsbmuxdConnection::default().await;
    if usbmuxd.is_err() {
        eprintln!("Failed to connect to usbmuxd: {:?}", usbmuxd.err());
        return Err("Failed to connect to usbmuxd".to_string());
    }
    let mut usbmuxd = usbmuxd.unwrap();

    let devs = usbmuxd.get_devices().await.unwrap();
    if devs.is_empty() {
        return Ok(vec![]);
    }

    let device_info_futures: Vec<_> = devs
        .iter()
        .map(|d| async move {
            let provider = d.to_provider(UsbmuxdAddr::from_env_var().unwrap(), "iloader");
            let device_uid = d.device_id;

            let mut lockdown_client = match LockdownClient::connect(&provider).await {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Unable to connect to lockdown: {e:?}");
                    return DeviceInfo {
                        name: String::from("Unknown Device"),
                        id: device_uid,
                        uuid: d.udid.clone(),
                    };
                }
            };

            let device_name = lockdown_client
                .get_value(Some("DeviceName"), None)
                .await
                .expect("Failed to get device name")
                .as_string()
                .expect("Failed to convert device name to string")
                .to_string();

            DeviceInfo {
                name: device_name,
                id: device_uid,
                uuid: d.udid.clone(),
            }
        })
        .collect();

    Ok(futures::future::join_all(device_info_futures).await)
}

#[tauri::command]
pub async fn set_selected_device(
    device_state: State<'_, DeviceInfoMutex>,
    device: Option<DeviceInfo>,
) -> Result<(), String> {
    let mut device_state = device_state.lock().unwrap();
    *device_state = device;
    Ok(())
}

pub async fn get_provider(device_info: &DeviceInfo) -> Result<UsbmuxdProvider, String> {
    let mut usbmuxd = UsbmuxdConnection::default()
        .await
        .map_err(|e| format!("Failed to connect to usbmuxd: {}", e))?;

    get_provider_from_connection(device_info, &mut usbmuxd).await
}

pub async fn get_provider_from_connection(
    device_info: &DeviceInfo,
    connection: &mut UsbmuxdConnection,
) -> Result<UsbmuxdProvider, String> {
    let device = connection
        .get_device(&device_info.uuid)
        .await
        .map_err(|e| format!("Failed to get device: {}", e))?;

    let provider = device.to_provider(UsbmuxdAddr::from_env_var().unwrap(), "iloader");
    Ok(provider)
}

pub async fn get_udid(
    device: &DeviceInfo,
    connection: &mut UsbmuxdConnection,
) -> Result<String, String> {
    let provider = get_provider_from_connection(device, connection).await?;
    let mut lockdown_client = LockdownClient::connect(&provider).await.map_err(|e| {
        format!(
            "Unable to connect to lockdown for device {}: {:?}",
            device.name, e
        )
    })?;
    let udid = lockdown_client
        .get_value(Some("UniqueDeviceID"), None)
        .await
        .map_err(|e| format!("Failed to get UDID for device {}: {:?}", device.name, e))?
        .as_string()
        .ok_or_else(|| {
            format!(
                "Failed to convert UDID to string for device {}",
                device.name
            )
        })?
        .to_string();
    Ok(udid)
}
