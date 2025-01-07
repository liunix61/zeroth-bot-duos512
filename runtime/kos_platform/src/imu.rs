use async_trait::async_trait;
use eyre::Result;
use kos_core::{
    google_proto::longrunning::Operation,
    hal::{EulerAnglesResponse, ImuValuesResponse, QuaternionResponse, IMU},
    kos_proto::common::{ActionResponse, Error, ErrorCode},
};
use linux_bno055::Bno055;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tracing::{debug, error, info};

pub struct ZBotIMU {
    imu: Arc<Mutex<Bno055>>,
}

impl ZBotIMU {
    pub fn new(i2c_bus: &str) -> Result<Self> {
        info!("Initializing ZerothIMU with I2C bus: {}", i2c_bus);
        
        let imu = Bno055::new(i2c_bus)?;
        
        Ok(Self {
            imu: Arc::new(Mutex::new(imu)),
        })
    }
}

impl Default for ZBotIMU {
    fn default() -> Self {
        unimplemented!("ZBotIMU cannot be default, it requires I2C bus configuration")
    }
}

#[async_trait]
impl IMU for ZBotIMU {
    async fn get_values(&self) -> Result<ImuValuesResponse> {
        let mut imu = self.imu.lock().await;
        
        let accel = imu.get_linear_acceleration()?;
        
        Ok(ImuValuesResponse {
            accel_x: accel.x as f64,
            accel_y: accel.y as f64,
            accel_z: accel.z as f64,
            gyro_x: 0.0, // Note: linux_bno055 doesn't expose raw gyro values in the example
            gyro_y: 0.0, // You may want to add these if needed
            gyro_z: 0.0,
            mag_x: None, // Similarly for magnetometer values
            mag_y: None,
            mag_z: None,
            error: None,
        })
    }

    async fn get_euler(&self) -> Result<EulerAnglesResponse> {
        let mut imu = self.imu.lock().await;
        let euler = imu.get_euler_angles()?;
        
        Ok(EulerAnglesResponse {
            roll: euler.roll as f64,
            pitch: euler.pitch as f64,
            yaw: euler.yaw as f64,
            error: None,
        })
    }

    async fn get_quaternion(&self) -> Result<QuaternionResponse> {
        let mut imu = self.imu.lock().await;
        let quat = imu.get_quaternion()?;
        
        Ok(QuaternionResponse {
            w: quat.w as f64,
            x: quat.x as f64,
            y: quat.y as f64,
            z: quat.z as f64,
            error: None,
        })
    }

    async fn calibrate(&self) -> Result<Operation> {
        info!("Starting IMU calibration");

        Ok(Operation {
            name: "operations/calibrate_imu/0".to_string(),
            metadata: None,
            done: true,
            result: None,
        })
    }

    async fn zero(
        &self,
        duration: Option<Duration>,
        max_retries: Option<u32>,
        max_angular_error: Option<f32>,
        max_vel: Option<f32>,
        max_accel: Option<f32>,
    ) -> Result<ActionResponse> {
        let mut imu = self.imu.lock().await;
        
        match imu.reset() {
            Ok(_) => {
                // Reset successful, now set mode back to NDOF
                if let Err(e) = imu.set_mode(linux_bno055::registers::OperationMode::Ndof) {
                    error!("Failed to set IMU mode after reset: {}", e);
                    return Ok(ActionResponse {
                        success: false,
                        error: Some(Error {
                            code: ErrorCode::HardwareFailure as i32,
                            message: format!("Failed to set IMU mode: {}", e),
                        }),
                    });
                }
                
                Ok(ActionResponse {
                    success: true,
                    error: None,
                })
            }
            Err(e) => {
                error!("Failed to zero IMU: {}", e);
                Ok(ActionResponse {
                    success: false,
                    error: Some(Error {
                        code: ErrorCode::HardwareFailure as i32,
                        message: format!("Failed to zero IMU: {}", e),
                    }),
                })
            }
        }
    }
}
