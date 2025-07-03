use crate::input::KeyboardState;
use ::input::Libinput;
use drm::control::atomic::AtomicModeReq;
pub use drm::control::Device as ControlDevice;
use drm::control::{
    self, connector, crtc, plane, property, AtomicCommitFlags, Mode, PageFlipFlags, PageFlipTarget,
};
pub use drm::Device;
use drm::VblankWaitFlags;
use gbm::{AsRaw, BufferObjectFlags};
use glutin::api::egl;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::ContextAttributesBuilder;
use glutin::display::{AsRawDisplay, RawDisplay};
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use input::{InputInterface, MouseState};
use libc::{c_char, c_int, c_short, ioctl, pollfd, SIGUSR1, SIGUSR2};
use raw_window_handle::{GbmDisplayHandle, GbmWindowHandle, RawDisplayHandle, RawWindowHandle};
use std::collections::HashMap;
use std::ffi::{c_void, CString};
use std::fs::OpenOptions;
use std::os::fd::{AsFd, AsRawFd};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::thread::sleep_ms;
use std::time::Duration;
use std::u64;

static TTY_FOCUS: AtomicBool = AtomicBool::new(true);
static TTY: LazyLock<std::fs::File> = LazyLock::new(|| {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .expect("Failed to open /dev/tty")
});

#[repr(C)]
struct vt_mode {
    mode: c_char,    // Operation mode
    waitv: c_char,   // Unused
    relsig: c_short, // Signal when releasing VT
    acqsig: c_short, // Signal when acquiring VT
    frsig: c_short,  // Unused
}

const VT_PROCESS: c_char = 0x01;
const VT_SETMODE: u64 = 0x5602; // from <linux/vt.h>
const VT_RELDISP: u64 = 0x5605; // from <linux/vt.h>

unsafe extern "C" fn handle_release(_sig: i32) {
    TTY_FOCUS.store(false, Ordering::Relaxed);
    libc::ioctl(TTY.as_raw_fd(), VT_RELDISP, 1);
    set_tty_text_mode(TTY.as_raw_fd())
        .map_err(|e| println!("Failed to set text mode: {e}"))
        .ok();
}

unsafe extern "C" fn handle_acquire(_sig: i32) {
    TTY_FOCUS.store(true, Ordering::Relaxed);
    set_tty_graphics_mode(TTY.as_raw_fd())
        .map_err(|e| println!("Failed to set graphics mode: {e}"))
        .ok();
}
const KDSETMODE: u64 = 0x4B3A; // from <linux/kd.h>
const KD_TEXT: c_int = 0;
const KD_GRAPHICS: c_int = 1;

fn set_tty_graphics_mode(fd: i32) -> std::io::Result<()> {
    let ret = unsafe { libc::ioctl(fd, KDSETMODE, KD_GRAPHICS) };
    if ret < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
fn set_tty_text_mode(fd: i32) -> std::io::Result<()> {
    let ret = unsafe { libc::ioctl(fd, KDSETMODE, KD_TEXT) };
    if ret < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
fn set_vt_mode(fd: i32) -> std::io::Result<()> {
    let mut vt = vt_mode {
        mode: VT_PROCESS,
        waitv: 0,
        relsig: SIGUSR1 as i16,
        acqsig: SIGUSR2 as i16,
        frsig: 0,
    };

    let ret = unsafe { ioctl(fd, VT_SETMODE, &mut vt) };
    if ret < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

use crate::gl;

use super::GlesContext;

#[derive(Debug)]
/// A simple wrapper for a device node.
pub struct Card(std::fs::File);

/// Implementing `AsFd` is a prerequisite to implementing the traits found
/// in this crate. Here, we are just calling `as_fd()` on the inner File.
impl std::os::unix::io::AsFd for Card {
    fn as_fd(&self) -> std::os::unix::io::BorrowedFd<'_> {
        self.0.as_fd()
    }
}

/// With `AsFd` implemented, we can now implement `drm::Device`.
impl Device for Card {}
impl ControlDevice for Card {}

/// Simple helper methods for opening a `Card`.
impl Card {
    pub fn open(path: &str) -> std::io::Result<Self> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        options.write(true);
        Ok(Self(options.open(path)?))
    }

    pub fn open_global() -> Self {
        let query = || {
            egl::device::Device::query_devices()
                .expect("Failed to query devices")
                .filter_map(|egl_device| {
                    egl_device
                        .drm_device_node_path()
                        .and_then(|p| p.as_os_str().to_str())
                })
                .chain(["/dev/dri/card0", "/dev/dri/card1"])
        };
        let mut devices = query();
        let started_time = std::time::Instant::now();
        loop {
            let Some(drm) = devices.next() else {
                if started_time.elapsed().as_secs() < 5 {
                    println!("Failed to find device, trying again in 50ms");
                    devices = query();
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                panic!("No device found (waited for 5s)");
            };
            match Self::open(drm) {
                Ok(card) => {
                    println!("Using device: {}", drm);
                    return card;
                }
                Err(e) => {
                    println!("Failed to open device {}: {}", drm, e);
                }
            }
        }
    }

    fn get_connector_and_crtc(&self) -> (connector::Info, crtc::Info, Mode, plane::Handle) {
        let res = self
            .resource_handles()
            .expect("Could not load normal resource ids.");
        let coninfo: Vec<connector::Info> = res
            .connectors()
            .iter()
            .flat_map(|con| self.get_connector(*con, true))
            .collect();

        let con = coninfo
            .iter()
            .find(|&i| i.state() == connector::State::Connected)
            .expect("No connected connectors");

        let crtcinfo: Vec<crtc::Info> = res
            .crtcs()
            .iter()
            .flat_map(|crtc| self.get_crtc(*crtc))
            .collect();
        let &mode = con.modes().first().expect("No modes found on connector");

        let crtc = crtcinfo.first().expect("No crtcs found");

        let planes = self.plane_handles().expect("Could not list planes");
        let plane = planes
            .into_iter()
            .find(|&plane| {
                self.get_plane(plane)
                    .map(|plane_info| {
                        let compatible_crtcs = res.filter_crtcs(plane_info.possible_crtcs());
                        if !compatible_crtcs.contains(&crtc.handle()) {
                            return false;
                        }
                        if let Ok(props) = self.get_properties(plane) {
                            for (&id, &val) in props.iter() {
                                if let Ok(info) = self.get_property(id) {
                                    if info.name().to_str().map(|x| x == "type").unwrap_or(false) {
                                        return val
                                            == (drm::control::PlaneType::Primary as u32).into();
                                    }
                                }
                            }
                        }
                        false
                    })
                    .unwrap_or(false)
            })
            .expect("Failed to find primary plane");

        (con.clone(), crtc.clone(), mode, plane)
    }
}
pub struct DrmContext {
    display: egl::display::Display,
    gbm_device: gbm::Device<Card>,
    gbm_surface: gbm::Surface<()>,
    surface: egl::surface::Surface<WindowSurface>,
    context: egl::context::PossiblyCurrentContext,
    connector: connector::Info,
    crtc: crtc::Info,
    mode: Mode,
    libinput: Libinput,
    xkb_state: xkbcommon::xkb::State,
    keyboard_state: KeyboardState,
    mouse_state: MouseState,
    focused: bool,
    plane: plane::Handle,
    plane_properties: HashMap<String, property::Info>,
    first_frame: bool,
}

fn find_egl_config(egl_display: &egl::display::Display) -> egl::config::Config {
    unsafe { egl_display.find_configs(ConfigTemplateBuilder::new().build()) }
        .unwrap()
        .reduce(|config, acc| {
            if config.num_samples() > acc.num_samples() {
                config
            } else {
                acc
            }
        })
        .expect("No available configs")
}

impl GlesContext for DrmContext {
    fn get_proc_address(&mut self, fn_name: &str) -> *const std::ffi::c_void {
        let symbol = CString::new(fn_name).unwrap();
        self.display.get_proc_address(symbol.as_c_str())
    }

    fn swap_buffers(&mut self) -> bool {
        self._swap_buffers()
            .map_err(|e| println!("Failed to swap buffers: {}", e))
            .is_ok()
    }

    fn size(&self) -> (u32, u32) {
        (self.mode.size().0 as u32, self.mode.size().1 as u32)
    }
}

impl DrmContext {
    pub fn new() -> Self {
        let card = Card::open_global();

        card.set_client_capability(drm::ClientCapability::UniversalPlanes, true)
            .expect("Unable to request UniversalPlanes capability");
        card.set_client_capability(drm::ClientCapability::Atomic, true)
            .expect("Unable to request Atomic capability");

        let (connector, crtc, mode, plane) = card.get_connector_and_crtc();
        let gbm = gbm::Device::new(card).unwrap();
        let (disp_width, disp_height) = mode.size();
        let rdh = RawDisplayHandle::Gbm(GbmDisplayHandle::new(
            NonNull::new(gbm.as_raw_mut()).unwrap().cast(),
        ));
        let egl_display = unsafe { egl::display::Display::new(rdh) }.expect("Create EGL Display");
        let config = find_egl_config(&egl_display);
        let gbm_surface = gbm
            .create_surface::<()>(
                disp_width.into(),
                disp_height.into(),
                gbm::Format::Xrgb8888,
                BufferObjectFlags::SCANOUT | BufferObjectFlags::RENDERING,
            )
            .unwrap();
        let rwh = RawWindowHandle::Gbm(GbmWindowHandle::new(
            NonNull::new(gbm_surface.as_raw_mut()).unwrap().cast(),
        ));
        let surface = unsafe {
            egl_display
                .create_window_surface(
                    &config,
                    &SurfaceAttributesBuilder::<WindowSurface>::new().build(
                        rwh,
                        (disp_width as u32).try_into().unwrap(),
                        (disp_height as u32).try_into().unwrap(),
                    ),
                )
                .expect("Failed to create EGL surface")
        };
        let context = unsafe {
            egl_display
                .create_context(&config, &ContextAttributesBuilder::new().build(Some(rwh)))
                .expect("Failed to create EGL context")
                .make_current(&surface)
                .unwrap()
        };
        let mut libinput = Libinput::new_with_udev(InputInterface);
        libinput.udev_assign_seat("seat0").unwrap();
        let xkb_context = xkbcommon::xkb::Context::new(xkbcommon::xkb::CONTEXT_NO_FLAGS);
        let xkb_keymap = xkbcommon::xkb::Keymap::new_from_names(
            &xkb_context,
            "evdev",
            "evdev",
            "",
            "",
            None,
            xkbcommon::xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .unwrap();
        let xkb_state = xkbcommon::xkb::State::new(&xkb_keymap);
        let tty_fd = TTY.as_raw_fd();
        set_vt_mode(tty_fd).expect("Failed to set VT mode");
        unsafe {
            libc::signal(SIGUSR1, handle_release as usize);
            libc::signal(SIGUSR2, handle_acquire as usize);
        }
        let mut context = DrmContext {
            display: egl_display,
            gbm_surface,
            surface,
            context,
            connector,
            crtc,
            mode,
            libinput,
            xkb_state,
            mouse_state: MouseState::new_at_middle(disp_width as u32, disp_height as u32),
            keyboard_state: KeyboardState::new(),
            focused: true,
            plane,
            plane_properties: gbm
                .get_properties(plane)
                .and_then(|p| p.as_hashmap(&gbm))
                .unwrap_or_default(),
            gbm_device: gbm,
            first_frame: true,
        };
        gl::load_with(|symbol| context.get_proc_address(symbol));
        context
    }

    fn _swap_buffers(&mut self) -> color_eyre::Result<()> {
        unsafe {
            self.surface.swap_buffers(&self.context)?;
            let frontbuffer = self.gbm_surface.lock_front_buffer()?;
            // if !self.first_frame {
            //     self.gbm_device.wait_vblank(
            //         drm::VblankWaitTarget::Relative(1),
            //         VblankWaitFlags::empty(),
            //         u32::from(self.crtc.handle()) >> 27,
            //         0,
            //     )?;
            // }
            let fb = self.gbm_device.add_framebuffer(&frontbuffer, 24, 32)?;
            let con_props = self
                .gbm_device
                .get_properties(self.connector.handle())?
                .as_hashmap(&self.gbm_device)?;
            let crtc_props = self
                .gbm_device
                .get_properties(self.crtc.handle())?
                .as_hashmap(&self.gbm_device)?;
            let plane = self.plane;
            let mut atomic_req = AtomicModeReq::new();

            atomic_req.add_property(
                self.connector.handle(),
                con_props["CRTC_ID"].handle(),
                property::Value::CRTC(Some(self.crtc.handle())),
            );

            let blob = self
                .gbm_device
                .create_property_blob(&self.mode)
                .expect("Failed to create blob");
            atomic_req.add_property(self.crtc.handle(), crtc_props["MODE_ID"].handle(), blob);
            atomic_req.add_property(
                self.crtc.handle(),
                crtc_props["ACTIVE"].handle(),
                property::Value::Boolean(true),
            );

            atomic_req.add_property(
                plane,
                self.plane_properties["FB_ID"].handle(),
                property::Value::Framebuffer(Some(fb)),
            );
            atomic_req.add_property(
                plane,
                self.plane_properties["CRTC_ID"].handle(),
                property::Value::CRTC(Some(self.crtc.handle())),
            );
            atomic_req.add_property(
                plane,
                self.plane_properties["SRC_X"].handle(),
                property::Value::UnsignedRange(0),
            );
            atomic_req.add_property(
                plane,
                self.plane_properties["SRC_Y"].handle(),
                property::Value::UnsignedRange(0),
            );
            atomic_req.add_property(
                plane,
                self.plane_properties["SRC_W"].handle(),
                property::Value::UnsignedRange((self.mode.size().0 as u64) << 16),
            );
            atomic_req.add_property(
                plane,
                self.plane_properties["SRC_H"].handle(),
                property::Value::UnsignedRange((self.mode.size().1 as u64) << 16),
            );
            atomic_req.add_property(
                plane,
                self.plane_properties["CRTC_X"].handle(),
                property::Value::SignedRange(0),
            );
            atomic_req.add_property(
                plane,
                self.plane_properties["CRTC_Y"].handle(),
                property::Value::SignedRange(0),
            );
            atomic_req.add_property(
                plane,
                self.plane_properties["CRTC_W"].handle(),
                property::Value::UnsignedRange(self.mode.size().0 as u64),
            );
            atomic_req.add_property(
                plane,
                self.plane_properties["CRTC_H"].handle(),
                property::Value::UnsignedRange(self.mode.size().1 as u64),
            );
            // Use atomic commit with PAGE_FLIP_EVENT
            self.gbm_device
                .atomic_commit(
                    AtomicCommitFlags::PAGE_FLIP_EVENT,
                    atomic_req,
                )
                .expect("Failed to commit atomic request");

            // // Now wait for the event from drm fd
            // let drm_fd = self.gbm_device.as_fd().as_raw_fd();
            // let mut fds = [pollfd {
            //   fd: drm_fd, events: libc::POLLIN,
            //   revents: 0
            // }];
            // libc::poll(fds.as_mut_ptr(), fds.len() as u64, -1); // Block until event

            // FIXME: Try to fix the stupid tearing, i'm already wanting to kill myself because I already spent
            // +4 hours trying to solve tearing I can't do this anymore
            'l: loop {
              for event in self.gbm_device.receive_events()? {
                  if let control::Event::PageFlip(_) = event {
                      break 'l; // Page flip event received
                  }
              }
            }
            self.first_frame = false;
            Ok(())
        }
    }
}

mod input;
