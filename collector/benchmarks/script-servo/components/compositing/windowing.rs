/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Abstract windowing methods. The concrete implementations of these can be found in `platform/`.

use compositor_thread::EventLoopWaker;
use euclid::{Point2D, Size2D};
use euclid::{ScaleFactor, TypedPoint2D, TypedSize2D};
use gleam::gl;
use ipc_channel::ipc::IpcSender;
use msg::constellation_msg::{Key, KeyModifiers, KeyState, TopLevelBrowsingContextId, TraversalDirection};
use net_traits::net_error_list::NetError;
use script_traits::{LoadData, MouseButton, TouchEventType, TouchId, TouchpadPressurePhase};
use servo_geometry::DeviceIndependentPixel;
use servo_url::ServoUrl;
use std::fmt::{Debug, Error, Formatter};
use std::rc::Rc;
use style_traits::DevicePixel;
use style_traits::cursor::Cursor;
use webrender_api::{DeviceUintSize, DeviceUintRect, ScrollLocation};

#[derive(Clone)]
pub enum MouseWindowEvent {
    Click(MouseButton, TypedPoint2D<f32, DevicePixel>),
    MouseDown(MouseButton, TypedPoint2D<f32, DevicePixel>),
    MouseUp(MouseButton, TypedPoint2D<f32, DevicePixel>),
}

/// Various debug and profiling flags that WebRender supports.
#[derive(Clone)]
pub enum WebRenderDebugOption {
    Profiler,
    TextureCacheDebug,
    RenderTargetDebug,
}

/// Events that the windowing system sends to Servo.
#[derive(Clone)]
pub enum WindowEvent {
    /// Sent when no message has arrived, but the event loop was kicked for some reason (perhaps
    /// by another Servo subsystem).
    ///
    /// FIXME(pcwalton): This is kind of ugly and may not work well with multiprocess Servo.
    /// It's possible that this should be something like
    /// `CompositorMessageWindowEvent(compositor_thread::Msg)` instead.
    Idle,
    /// Sent when part of the window is marked dirty and needs to be redrawn. Before sending this
    /// message, the window must make the same GL context as in `PrepareRenderingEvent` current.
    Refresh,
    /// Sent when the window is resized.
    Resize(DeviceUintSize),
    /// Touchpad Pressure
    TouchpadPressure(TypedPoint2D<f32, DevicePixel>, f32, TouchpadPressurePhase),
    /// Sent when a new URL is to be loaded.
    LoadUrl(TopLevelBrowsingContextId, ServoUrl),
    /// Sent when a mouse hit test is to be performed.
    MouseWindowEventClass(MouseWindowEvent),
    /// Sent when a mouse move.
    MouseWindowMoveEventClass(TypedPoint2D<f32, DevicePixel>),
    /// Touch event: type, identifier, point
    Touch(TouchEventType, TouchId, TypedPoint2D<f32, DevicePixel>),
    /// Sent when the user scrolls. The first point is the delta and the second point is the
    /// origin.
    Scroll(ScrollLocation, TypedPoint2D<i32, DevicePixel>, TouchEventType),
    /// Sent when the user zooms.
    Zoom(f32),
    /// Simulated "pinch zoom" gesture for non-touch platforms (e.g. ctrl-scrollwheel).
    PinchZoom(f32),
    /// Sent when the user resets zoom to default.
    ResetZoom,
    /// Sent when the user uses chrome navigation (i.e. backspace or shift-backspace).
    Navigation(TopLevelBrowsingContextId, TraversalDirection),
    /// Sent when the user quits the application
    Quit,
    /// Sent when a key input state changes
    KeyEvent(Option<char>, Key, KeyState, KeyModifiers),
    /// Sent when Ctr+R/Apple+R is called to reload the current page.
    Reload(TopLevelBrowsingContextId),
    /// Create a new top level browsing context
    NewBrowser(ServoUrl, IpcSender<TopLevelBrowsingContextId>),
    /// Close a top level browsing context
    CloseBrowser(TopLevelBrowsingContextId),
    /// Make a top level browsing context visible, hiding the previous
    /// visible one.
    SelectBrowser(TopLevelBrowsingContextId),
    /// Toggles a debug flag in WebRender
    ToggleWebRenderDebug(WebRenderDebugOption),
}

impl Debug for WindowEvent {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match *self {
            WindowEvent::Idle => write!(f, "Idle"),
            WindowEvent::Refresh => write!(f, "Refresh"),
            WindowEvent::Resize(..) => write!(f, "Resize"),
            WindowEvent::TouchpadPressure(..) => write!(f, "TouchpadPressure"),
            WindowEvent::KeyEvent(..) => write!(f, "Key"),
            WindowEvent::LoadUrl(..) => write!(f, "LoadUrl"),
            WindowEvent::MouseWindowEventClass(..) => write!(f, "Mouse"),
            WindowEvent::MouseWindowMoveEventClass(..) => write!(f, "MouseMove"),
            WindowEvent::Touch(..) => write!(f, "Touch"),
            WindowEvent::Scroll(..) => write!(f, "Scroll"),
            WindowEvent::Zoom(..) => write!(f, "Zoom"),
            WindowEvent::PinchZoom(..) => write!(f, "PinchZoom"),
            WindowEvent::ResetZoom => write!(f, "ResetZoom"),
            WindowEvent::Navigation(..) => write!(f, "Navigation"),
            WindowEvent::Quit => write!(f, "Quit"),
            WindowEvent::Reload(..) => write!(f, "Reload"),
            WindowEvent::NewBrowser(..) => write!(f, "NewBrowser"),
            WindowEvent::CloseBrowser(..) => write!(f, "CloseBrowser"),
            WindowEvent::SelectBrowser(..) => write!(f, "SelectBrowser"),
            WindowEvent::ToggleWebRenderDebug(..) => write!(f, "ToggleWebRenderDebug"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimationState {
    Idle,
    Animating,
}

pub trait WindowMethods {
    /// Returns the rendering area size in hardware pixels.
    fn framebuffer_size(&self) -> DeviceUintSize;
    /// Returns the position and size of the window within the rendering area.
    fn window_rect(&self) -> DeviceUintRect;
    /// Returns the size of the window in density-independent "px" units.
    fn size(&self) -> TypedSize2D<f32, DeviceIndependentPixel>;
    /// Presents the window to the screen (perhaps by page flipping).
    fn present(&self);

    /// Return the size of the window with head and borders and position of the window values
    fn client_window(&self, ctx: TopLevelBrowsingContextId) -> (Size2D<u32>, Point2D<i32>);
    /// Set the size inside of borders and head
    fn set_inner_size(&self, ctx: TopLevelBrowsingContextId, size: Size2D<u32>);
    /// Set the window position
    fn set_position(&self, ctx: TopLevelBrowsingContextId, point: Point2D<i32>);
    /// Set fullscreen state
    fn set_fullscreen_state(&self, ctx: TopLevelBrowsingContextId, state: bool);

    /// Sets the page title for the current page.
    fn set_page_title(&self, ctx: TopLevelBrowsingContextId, title: Option<String>);
    /// Called when the browser chrome should display a status message.
    fn status(&self, ctx: TopLevelBrowsingContextId, Option<String>);
    /// Called when the browser has started loading a frame.
    fn load_start(&self, ctx: TopLevelBrowsingContextId);
    /// Called when the browser is done loading a frame.
    fn load_end(&self, ctx: TopLevelBrowsingContextId);
    /// Called when the browser encounters an error while loading a URL
    fn load_error(&self, ctx: TopLevelBrowsingContextId, code: NetError, url: String);
    /// Wether or not to follow a link
    fn allow_navigation(&self, ctx: TopLevelBrowsingContextId, url: ServoUrl, IpcSender<bool>);
    /// Called when the <head> tag has finished parsing
    fn head_parsed(&self, ctx: TopLevelBrowsingContextId);
    /// Called when the history state has changed.
    fn history_changed(&self, ctx: TopLevelBrowsingContextId, Vec<LoadData>, usize);

    /// Returns the scale factor of the system (device pixels / device independent pixels).
    fn hidpi_factor(&self) -> ScaleFactor<f32, DeviceIndependentPixel, DevicePixel>;

    /// Returns a thread-safe object to wake up the window's event loop.
    fn create_event_loop_waker(&self) -> Box<EventLoopWaker>;

    /// Requests that the window system prepare a composite. Typically this will involve making
    /// some type of platform-specific graphics context current. Returns true if the composite may
    /// proceed and false if it should not.
    fn prepare_for_composite(&self, width: usize, height: usize) -> bool;

    /// Sets the cursor to be used in the window.
    fn set_cursor(&self, cursor: Cursor);

    /// Process a key event.
    fn handle_key(&self, ctx: Option<TopLevelBrowsingContextId>, ch: Option<char>, key: Key, mods: KeyModifiers);

    /// Does this window support a clipboard
    fn supports_clipboard(&self) -> bool;

    /// Add a favicon
    fn set_favicon(&self, ctx: TopLevelBrowsingContextId, url: ServoUrl);

    /// Return the GL function pointer trait.
    fn gl(&self) -> Rc<gl::Gl>;

    /// Set whether the application is currently animating.
    /// Typically, when animations are active, the window
    /// will want to avoid blocking on UI events, and just
    /// run the event loop at the vsync interval.
    fn set_animation_state(&self, _state: AnimationState) {}
}
