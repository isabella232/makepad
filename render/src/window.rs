use crate::cx::*;
use std::rc::Rc;
use std::cell::RefCell;

pub struct Window {
    pub window_id: usize,
    pub windows_free: Rc<RefCell<Vec<usize>>>,
}

impl Drop for Window{
    fn drop(&mut self){
        self.windows_free.borrow_mut().push(self.window_id)
    }
}

impl Window {
    pub fn new(cx: &mut Cx) -> Self {
        let windows_free = cx.windows_free.clone();
        let window_id =  if let Some(view_id) = windows_free.borrow_mut().pop(  ){
            view_id 
        }
        else{
            let window_id = cx.windows.len();
            cx.windows.push(CxWindow {
                window_state: CxWindowState::Create {
                    title: "Fill in here".to_string(),
                    inner_size: cx.get_default_window_size(),
                    position: None
                },
                ..Default::default()
            });
            window_id
        };
        
        Self {
            windows_free,
            window_id,
        }
    }
    
    pub fn begin_window(&mut self, cx: &mut Cx) {
        // if we are not at ground level for viewports,
        cx.windows[self.window_id].main_pass_id = None;
        cx.window_stack.push(self.window_id);
        
    }
    
    pub fn get_inner_size(&mut self, cx: &mut Cx) -> Vec2 {
        return cx.windows[self.window_id].get_inner_size()
    }
    
    pub fn get_position(&mut self, cx: &mut Cx) -> Option<Vec2> {
        return cx.windows[self.window_id].get_position()
    }
    
        
    pub fn set_position(&mut self, cx: &mut Cx, pos:Vec2) {
        return cx.windows[self.window_id].window_set_position = Some(pos);
    }
    
    pub fn handle_window(&mut self, _cx: &mut Cx, _event: &mut Event) -> bool {
        false
    }
    /*
    pub fn redraw_window_area(&mut self, cx: &mut Cx) {
        if let Some(pass_id) = cx.windows[self.window_id].main_pass_id {
            cx.redraw_pass_and_sub_passes(pass_id);
        }
    }*/
    
    pub fn end_window(&mut self, cx: &mut Cx) -> Area {
        cx.window_stack.pop();
        Area::Empty
    }
    
    pub fn minimize_window(&mut self, cx: &mut Cx) {
        cx.windows[self.window_id].window_command = CxWindowCmd::Minimize;
    }
    
    pub fn maximize_window(&mut self, cx: &mut Cx) {
        cx.windows[self.window_id].window_command = CxWindowCmd::Maximize;
    }
    
    pub fn fullscreen_window(&mut self, cx:&mut Cx){
        cx.windows[self.window_id].window_command = CxWindowCmd::FullScreen;
    }
    
    pub fn normal_window(&mut self, cx:&mut Cx){
        cx.windows[self.window_id].window_command = CxWindowCmd::NormalScreen;
    }
    
        
    pub fn can_fullscreen(&mut self, cx: &mut Cx) -> bool {
        cx.windows[self.window_id].window_geom.can_fullscreen
    }
    
    pub fn xr_can_present(&mut self, cx: &mut Cx) -> bool {
        cx.windows[self.window_id].window_geom.xr_can_present
    }
    
    pub fn is_fullscreen(&mut self, cx: &mut Cx) -> bool {
        cx.windows[self.window_id].window_geom.is_fullscreen
    }
    
    pub fn xr_is_presenting(&mut self, cx: &mut Cx) -> bool {
        cx.windows[self.window_id].window_geom.xr_is_presenting
    }
    
    pub fn xr_start_presenting(&mut self, cx: &mut Cx){
        cx.windows[self.window_id].window_command = CxWindowCmd::XrStartPresenting;
    }
    
    pub fn xr_stop_presenting(&mut self, cx: &mut Cx){
        cx.windows[self.window_id].window_command = CxWindowCmd::XrStopPresenting;
    }
    
    pub fn is_topmost(&mut self, cx: &mut Cx) -> bool {
        cx.windows[self.window_id].window_geom.is_topmost
    }
    
    pub fn set_topmost(&mut self, cx: &mut Cx, topmost: bool) {
        cx.windows[self.window_id].window_topmost = Some(topmost);
    }
    
    pub fn restore_window(&mut self, cx: &mut Cx) {
        cx.windows[self.window_id].window_command = CxWindowCmd::Restore;
    }
    
    pub fn close_window(&mut self, cx: &mut Cx) {
        cx.windows[self.window_id].window_state = CxWindowState::Close;
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindowGeom {
    pub dpi_factor: f32,
    pub can_fullscreen: bool,
    pub xr_can_present: bool,
    pub xr_is_presenting: bool,
    pub is_fullscreen: bool,
    pub is_topmost: bool,
    pub position: Vec2,
    pub inner_size: Vec2,
    pub outer_size: Vec2,
}

#[derive(Clone)]
pub enum CxWindowState {
    Create {title: String, inner_size: Vec2, position: Option<Vec2>},
    Created,
    Close,
    Closed
}

#[derive(Clone)]
pub enum CxWindowCmd {
    None,
    Restore,
    Maximize,
    Minimize,
    XrStartPresenting,
    XrStopPresenting,
    FullScreen,
    NormalScreen
}

impl Default for CxWindowCmd {
    fn default() -> Self {CxWindowCmd::None}
}

impl Default for CxWindowState {
    fn default() -> Self {CxWindowState::Closed}
}

#[derive(Clone, Default)]
pub struct CxWindow {
    pub window_state: CxWindowState,
    pub window_command: CxWindowCmd,
    pub window_set_position: Option<Vec2>,
    pub window_topmost: Option<bool>,
    pub window_geom: WindowGeom,
    pub main_pass_id: Option<usize>,
}

impl CxWindow {
    pub fn get_inner_size(&mut self) -> Vec2 {
        match &self.window_state {
            CxWindowState::Create {inner_size, ..} => *inner_size,
            CxWindowState::Created => self.window_geom.inner_size,
            _ => Vec2::default()
        }
    }
    
    pub fn get_position(&mut self) -> Option<Vec2> {
        match &self.window_state {
            CxWindowState::Create {position, ..} => *position,
            CxWindowState::Created => Some(self.window_geom.position),
            _ => None
        }
    }
    
    pub fn get_dpi_factor(&mut self) -> Option<f32> {
        match &self.window_state {
            CxWindowState::Created => Some(self.window_geom.dpi_factor),
            _ => None
        }
    }
}