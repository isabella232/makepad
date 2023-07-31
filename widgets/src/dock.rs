use {
    crate::{
        makepad_derive_widget::*,
        widget::*,
        makepad_draw::*,
        splitter::{SplitterAction, Splitter, SplitterAlign},
        tab_bar::{TabBarAction, TabBar},
    },
};

live_design!{
    import makepad_draw::shader::std::*;
    import crate::tab_bar::TabBar
    import makepad_widgets::splitter::Splitter
    import makepad_widgets::theme::*;
    
    DrawRoundCorner = {{DrawRoundCorner}} {
        draw_depth: 6.0
        border_radius: 10.0
        fn pixel(self) -> vec4 {
            
            let pos = vec2(
                mix(self.pos.x, 1.0 - self.pos.x, self.flip.x),
                mix(self.pos.y, 1.0 - self.pos.y, self.flip.y)
            )
            
            let sdf = Sdf2d::viewport(pos * self.rect_size);
            sdf.rect(-10., -10., self.rect_size.x * 2.0, self.rect_size.y * 2.0);
            sdf.box(
                0.25,
                0.25,
                self.rect_size.x * 2.0,
                self.rect_size.y * 2.0,
                4.0
            );
            
            sdf.subtract()
            return sdf.fill(COLOR_BG_APP);
        }
    }
    
    const BORDER_SIZE: 6.0
    
    Dock = {{Dock}} {
        border_size: (BORDER_SIZE)
        layout: {
            flow: Down
            padding: {left: (BORDER_SIZE), top: 0.0, right: (BORDER_SIZE), bottom: (BORDER_SIZE)}
        }
        padding_fill: {color: (COLOR_BG_APP)}
        drag_quad: {
            draw_depth: 10.0
            color: (COLOR_DRAG_QUAD)
        }
        overlay_view: {
            //walk: {abs_pos: vec2(0.0, 0.0)}
            //is_overlay: true
        }
        tab_bar: <TabBar> {}
        splitter: <Splitter> {}
    }
}

#[derive(Live, LiveHook)]
#[repr(C)]
pub struct DrawRoundCorner {
    #[deref] draw_super: DrawQuad,
    #[live] border_radius: f32,
    #[live] flip: Vec2,
}

impl DrawRoundCorner {
    fn draw_corners(&mut self, cx: &mut Cx2d, rect: Rect) {
        self.flip = vec2(0.0, 0.0);
        let rad = dvec2(self.border_radius as f64, self.border_radius as f64);
        let pos = rect.pos;
        let size = rect.size;
        self.draw_abs(cx, Rect {pos, size: rad});
        self.flip = vec2(1.0, 0.0);
        self.draw_abs(cx, Rect {pos: pos + dvec2(size.x - rad.x, 0.), size: rad});
        self.flip = vec2(1.0, 1.0);
        self.draw_abs(cx, Rect {pos: pos + dvec2(size.x - rad.x, size.y - rad.y), size: rad});
        self.flip = vec2(0.0, 1.0);
        self.draw_abs(cx, Rect {pos: pos + dvec2(0., size.y - rad.y), size: rad});
    }
}

#[derive(Live)]
pub struct Dock {
    #[rust] draw_state: DrawStateWrap<DockDrawState>,
    #[live] walk: Walk,
    #[live] layout: Layout,
    #[live] drag_view: View,
    #[live] round_corner: DrawRoundCorner,
    #[live] padding_fill: DrawColor,
    #[live] border_size: f64,
    #[live] drag_quad: DrawColor,
    
    #[live] tab_bar: Option<LivePtr>,
    #[live] splitter: Option<LivePtr>,
    
    #[rust] area: Area,
    
    #[rust] tab_bars: ComponentMap<LiveId, TabBarWrap>,
    #[rust] splitters: ComponentMap<LiveId, Splitter>,
    
    #[rust] dock_items: ComponentMap<LiveId, DockItem>,
    #[rust] templates: ComponentMap<LiveId, LivePtr>,
    #[rust] items: ComponentMap<(LiveId, LiveId), WidgetRef>,
    
    #[rust] drag: Option<Drag>,
}

struct TabBarWrap {
    tab_bar: TabBar,
    contents_view: View,
    contents_rect: Rect
}

#[derive(Copy, Debug, Clone)]
enum DrawStackItem {
    Invalid,
    SplitLeft {id: LiveId},
    SplitRight {id: LiveId},
    SplitEnd {id: LiveId},
    Tabs {id: LiveId},
    TabLabel {id: LiveId, index: usize},
    Tab {id: LiveId},
    TabContent {id: LiveId}
}

impl DrawStackItem {
    fn from_dock_item(id: LiveId, dock_item: Option<&DockItem>) -> Self {
        match dock_item {
            None => DrawStackItem::Invalid,
            Some(DockItem::Splitter {..}) => {
                DrawStackItem::SplitLeft {id}
            }
            Some(DockItem::Tabs {..}) => {
                DrawStackItem::Tabs {id}
            }
            Some(DockItem::Tab {..}) => {
                DrawStackItem::Tab {id}
            }
        }
    }
}

#[derive(Clone)]
enum DockDrawState {
    Panels {
        stack: Vec<DrawStackItem>
    },
}

#[derive(Clone, Debug, Live, LiveHook)]
#[live_ignore]
pub enum DockItem {
    #[live {axis: Axis::Vertical, align: SplitterAlign::Weighted(0.5), a: LiveId(0), b: LiveId(0)}]
    Splitter {
        axis: Axis,
        align: SplitterAlign,
        a: LiveId,
        b: LiveId
    },
    #[live {tabs: vec![], selected: 0}]
    Tabs {
        tabs: Vec<LiveId>,
        selected: usize
    },
    #[pick {name: "Tab".to_string(), kind: LiveId(0), no_close: false}]
    Tab {
        name: String,
        no_close: bool,
        kind: LiveId
    }
}

impl LiveHook for Dock {
    fn apply_value_instance(&mut self, cx: &mut Cx, from: ApplyFrom, index: usize, nodes: &[LiveNode]) -> usize {
        let id = nodes[index].id;
        match from {
            ApplyFrom::NewFromDoc {file_id} | ApplyFrom::UpdateFromDoc {file_id} => {
                if nodes[index].origin.has_prop_type(LivePropType::Instance) {
                    if nodes[index].value.is_enum() {
                        let mut dock_item = DockItem::new(cx);
                        let index = dock_item.apply(cx, from, index, nodes);
                        self.dock_items.insert(id, dock_item);
                        return index;
                    }
                    else {
                        let live_ptr = cx.live_registry.borrow().file_id_index_to_live_ptr(file_id, index);
                        self.templates.insert(id, live_ptr);
                        // lets apply this thing over all our childnodes with that template
                        for ((_, templ_id), node) in self.items.iter_mut() {
                            if *templ_id == id {
                                node.apply(cx, from, index, nodes);
                            }
                        }
                    }
                }
                else {
                    cx.apply_error_no_matching_field(live_error_origin!(), index, nodes);
                }
            }
            _ => ()
        }
        nodes.skip_node(index)
    }
    fn before_live_design(cx: &mut Cx) {
        register_widget!(cx, Dock)
    }
}

impl Dock {
    
    fn begin(&mut self, cx: &mut Cx2d, walk: Walk) {
        cx.begin_turtle(walk, self.layout);
    }
    
    fn end(&mut self, cx: &mut Cx2d) {
        /*
        if self.drag_view.begin(cx, Walk::default()).is_redrawing() {
            if let Some(drag) = self.drag.as_ref() {
                let tab_bar = &self.tab_bars[drag.panel_id];
                let rect = compute_drag_rect(tab_bar.contents_rect, drag.position);
                self.drag_quad.draw_abs(cx, rect);
            }
            self.overlay_view.end(cx);
        }*/
        
        self.tab_bars.retain_visible();
        self.splitters.retain_visible();
        
        // lets draw the corners here
        for splitter in self.splitters.values() {
            self.round_corner.draw_corners(cx, splitter.area_a().get_rect(cx));
            self.round_corner.draw_corners(cx, splitter.area_b().get_rect(cx));
        }
        self.round_corner.draw_corners(cx, cx.turtle().rect());
        
        cx.end_turtle_with_area(&mut self.area);
    }
    
    
    pub fn get_item(&mut self, cx: &mut Cx2d, entry_id: LiveId, template: LiveId) -> Option<WidgetRef> {
        if let Some(ptr) = self.templates.get(&template) {
            let entry = self.items.get_or_insert(cx, (entry_id, template), | cx | {
                WidgetRef::new_from_ptr(cx, Some(*ptr))
            });
            return Some(entry.clone())
        }
        None
    }
    
    fn set_parent_split(&mut self, what_item: LiveId, replace_item: LiveId) {
        for item in self.dock_items.values_mut() {
            match item {
                DockItem::Splitter {a, b, ..} => {
                    if what_item == *a {
                        *a = replace_item;
                        return
                    }
                    else if what_item == *b {
                        *b = replace_item;
                        return
                    }
                }
                _ => ()
            }
        }
    }
    
    fn redraw_item(&mut self, cx:&mut Cx, item_id: LiveId) {
        if let Some(tab_bar) = self.tab_bars.get_mut(&item_id){
            tab_bar.contents_view.redraw(cx);
        }
        for ((id, kind), item) in self.items.iter_mut(){
            if *id == item_id{
                item.redraw(cx);
            }
        }
    }
    
    fn unsplit_tabs(&mut self,cx: &mut Cx,  tabs_id: LiveId) {
        for (splitter_id, item) in self.dock_items.iter_mut() {
            match *item {
                DockItem::Splitter {a, b, ..} => {
                    let splitter_id = *splitter_id;
                    if tabs_id == a {
                        self.set_parent_split(splitter_id, b);
                        self.dock_items.remove(&splitter_id);
                        self.dock_items.remove(&tabs_id);
                        self.redraw_item(cx, b);
                        return
                    }
                    else if tabs_id == b {
                        self.set_parent_split(splitter_id, a);
                        self.dock_items.remove(&splitter_id);
                        self.dock_items.remove(&tabs_id);
                        self.redraw_item(cx, a);
                        return
                    }
                }
                _ => ()
            }
        }
    }
    
    fn select_tab(&mut self, cx: &mut Cx, tab_id: LiveId) {
        for (tabs_id, item) in self.dock_items.iter_mut() {
            match item {
                DockItem::Tabs {tabs, selected} => if let Some(pos) = tabs.iter().position( | v | *v == tab_id) {
                    *selected = pos;
                    // ok now lets redraw the area of the tab
                    self.tab_bars.get_mut(&tabs_id).unwrap().contents_view.redraw(cx);
                }
                _ => ()
            }
        }
    }
    
    fn close_tab(&mut self, cx: &mut Cx, tab_id: LiveId) {
        // ok so we have to find the tab id in our tab bars / tabs and remove it
        // if we are the last tab we need to remove a splitter
        for (tabs_id, item) in self.dock_items.iter_mut() {
            match item {
                DockItem::Tabs {tabs, selected} => if let Some(pos) = tabs.iter().position( | v | *v == tab_id) {
                    // remove from the tabs array
                    let tabs_id = *tabs_id;
                    tabs.remove(pos);
                    if tabs.len() == 0 { // unsplit
                        self.unsplit_tabs(cx, tabs_id);
                    }
                    else{
                        let next_tab = if *selected >= tabs.len(){tabs[*selected - 1]} else {tabs[*selected]};
                        self.select_tab(cx, next_tab);
                        self.dock_items.remove(&tab_id);
                        self.area.redraw(cx);
                    }
                    return
                }
                _ => ()
            }
        }
    }
    
}


impl Widget for Dock {
    fn redraw(&mut self, cx: &mut Cx) {
        self.area.redraw(cx);
    }
    
    fn handle_widget_event_with(&mut self, cx: &mut Cx, event: &Event, dispatch_action: &mut dyn FnMut(&mut Cx, WidgetActionItem)) {
        // call handle on all tab bars, splitters,
        let uid = self.widget_uid();
        let dock_items = &mut self.dock_items;
        for (panel_id, splitter) in self.splitters.iter_mut() {
            splitter
                .handle_event_with(cx, event, &mut | cx, action | match action {
                SplitterAction::Changed {axis, align} => {
                    // lets move the splitter
                    if let Some(DockItem::Splitter {axis: _axis, align: _align, ..}) = dock_items.get_mut(&panel_id) {
                        *_axis = axis;
                        *_align = align;
                    }
                    dispatch_action(cx, DockAction::SplitPanelChanged {panel_id: *panel_id, axis, align}.into_action(uid));
                },
                _ => ()
            });
        }
        for (panel_id, tab_bar) in self.tab_bars.iter_mut() {
            let contents_view = &mut tab_bar.contents_view;
            for action in tab_bar.tab_bar.handle_event(cx, event) {
                match action {
                    TabBarAction::ReceivedDraggedItem(item) => dispatch_action(
                        cx,
                        DockAction::TabBarReceivedDraggedItem(*panel_id, item).into_action(uid),
                    ),
                    TabBarAction::TabWasPressed(tab_id) => {
                        if let Some(DockItem::Tabs {tabs, selected, ..}) = dock_items.get_mut(&panel_id) {
                            if let Some(sel) = tabs.iter().position( | v | *v == tab_id) {
                                *selected = sel;
                                contents_view.redraw(cx);
                                dispatch_action(cx, DockAction::TabWasPressed(tab_id).into_action(uid))
                            }
                            else {
                                log!("Cannot find tab {}", tab_id.0);
                            }
                        }
                    }
                    TabBarAction::TabCloseWasPressed(tab_id) => {
                        // alright.. we wanna remove the tab
                        
                        dispatch_action(cx, DockAction::TabCloseWasPressed(tab_id).into_action(uid))
                    }
                    TabBarAction::TabReceivedDraggedItem(tab_id, item) => {
                        dispatch_action(
                            cx,
                            DockAction::TabReceivedDraggedItem(tab_id, item).into_action(uid),
                        )
                    }
                }
            };
        }
        for item in self.items.values_mut() {
            item.handle_widget_event_with(cx, event, dispatch_action);
        }
        match event {
            Event::Drag(event) => {
                /*
                self.drag = None;
                for (panel_id, tab_bar) in self.tab_bars.iter_mut() {
                    if tab_bar.contents_rect.contains(event.abs) {
                        self.drag = Some(Drag {
                            panel_id: *panel_id,
                            position: compute_drag_position(tab_bar.contents_rect, event.abs),
                        });
                        event.action.set(DragAction::Copy);
                    }
                }
                self.overlay_view.redraw(cx);*/
            }
            Event::Drop(event) => {
                /*
                self.drag = None;
                for (panel_id, tab_bar) in self.tab_bars.iter_mut() {
                    if tab_bar.contents_rect.contains(event.abs) {
                        dispatch_action(
                            cx,
                            DockAction::ContentsReceivedDraggedItem(
                                *panel_id,
                                compute_drag_position(tab_bar.contents_rect, event.abs),
                                event.dragged_item.clone(),
                            ).into_action(uid),
                        );
                    }
                }
                self.overlay_view.redraw(cx);*/
            }
            _ => {}
        }
        
    }
    
    fn find_widgets(&mut self, path: &[LiveId], cached: WidgetCache, results: &mut WidgetSet) {
        if let Some(DockItem::Tab {kind, ..}) = self.dock_items.get(&path[0]) {
            if let Some(widget) = self.items.get_mut(&(path[0], *kind)) {
                if path.len()>1 {
                    widget.find_widgets(&path[1..], cached, results);
                }
                else {
                    results.push(widget.clone());
                }
            }
        }
        else {
            for widget in self.items.values_mut() {
                widget.find_widgets(path, cached, results);
            }
        }
    }
    
    fn get_walk(&self) -> Walk {self.walk}
    
    fn draw_walk_widget(&mut self, cx: &mut Cx2d, walk: Walk) -> WidgetDraw {
        if self.draw_state.begin_with(cx, &self.dock_items, | _, dock_items | {
            let id = live_id!(root);
            DockDrawState::Panels {
                stack: vec![DrawStackItem::from_dock_item(id, dock_items.get(&id))]
            }
        }) {
            self.begin(cx, walk);
        }
        
        while let Some(DockDrawState::Panels {stack}) = self.draw_state.as_mut() {
            match stack.pop() {
                Some(DrawStackItem::SplitLeft {id}) => {
                    stack.push(DrawStackItem::SplitRight {id});
                    // top becomes splitleft
                    let splitter = self.splitter;
                    let splitter = self.splitters.get_or_insert(cx, id, | cx | {
                        Splitter::new_from_ptr(cx, splitter)
                    });
                    if let Some(DockItem::Splitter {axis, align, a, ..}) = self.dock_items.get(&id) {
                        splitter.set_axis(*axis);
                        splitter.set_align(*align);
                        splitter.begin(cx, Walk::default());
                        stack.push(DrawStackItem::from_dock_item(*a, self.dock_items.get(&a)));
                        continue;
                    }
                    else {panic!()}
                }
                Some(DrawStackItem::SplitRight {id}) => {
                    stack.push(DrawStackItem::SplitEnd {id});
                    let splitter = self.splitters.get_mut(&id).unwrap();
                    splitter.middle(cx);
                    if let Some(DockItem::Splitter {b, ..}) = self.dock_items.get(&id) {
                        stack.push(DrawStackItem::from_dock_item(*b, self.dock_items.get(&b)));
                        continue;
                    }
                    else {panic!()}
                }
                Some(DrawStackItem::SplitEnd {id}) => {
                    let splitter = self.splitters.get_mut(&id).unwrap();
                    splitter.end(cx);
                }
                Some(DrawStackItem::Tabs {id}) => {
                    if let Some(DockItem::Tabs {selected, ..}) = self.dock_items.get(&id) {
                        // lets draw the tabs
                        let tab_bar = self.tab_bar;
                        let tab_bar = self.tab_bars.get_or_insert(cx, id, | cx | {
                            TabBarWrap {
                                tab_bar: TabBar::new_from_ptr(cx, tab_bar),
                                contents_view: View::new(cx),
                                contents_rect: Rect::default(),
                                //full_rect: Rect::default(),
                            }
                        });
                        tab_bar.tab_bar.begin(cx, Some(*selected));
                        stack.push(DrawStackItem::TabLabel {id, index: 0});
                    }
                    else {panic!()}
                }
                Some(DrawStackItem::TabLabel {id, index}) => {
                    if let Some(DockItem::Tabs {tabs, selected}) = self.dock_items.get(&id) {
                        let tab_bar = self.tab_bars.get_mut(&id).unwrap();
                        if index < tabs.len() {
                            if let Some(DockItem::Tab {name, ..}) = self.dock_items.get(&tabs[index]) {
                                tab_bar.tab_bar.draw_tab(cx, tabs[index].into(), name);
                            }
                            stack.push(DrawStackItem::TabLabel {id, index: index + 1});
                        }
                        else {
                            tab_bar.tab_bar.end(cx);
                            tab_bar.contents_rect = cx.turtle().rect();
                            if tab_bar.contents_view.begin(cx, Walk::default()).is_redrawing() {
                                stack.push(DrawStackItem::TabContent {id});
                                stack.push(DrawStackItem::Tab {id: tabs[*selected]});
                            }
                        }
                    }
                    else {panic!()}
                }
                Some(DrawStackItem::Tab {id}) => {
                    stack.push(DrawStackItem::Tab {id});
                    if let Some(DockItem::Tab {kind, ..}) = self.dock_items.get(&id) {
                        if let Some(ptr) = self.templates.get(&kind) {
                            let entry = self.items.get_or_insert(cx, (id, *kind), | cx | {
                                WidgetRef::new_from_ptr(cx, Some(*ptr))
                            });
                            entry.draw_widget(cx) ?;
                        }
                    }
                    stack.pop();
                }
                Some(DrawStackItem::TabContent {id}) => {
                    if let Some(DockItem::Tabs {..}) = self.dock_items.get(&id) {
                        let tab_bar = self.tab_bars.get_mut(&id).unwrap();
                        tab_bar.contents_view.end(cx);
                    }
                    else {panic!()}
                }
                Some(DrawStackItem::Invalid) => {}
                None => {
                    break
                }
            }
        }
        
        self.end(cx);
        self.draw_state.end();
        
        WidgetDraw::done()
    }
}

/*
#[derive(Clone, Debug, Default, Eq, Hash, Copy, PartialEq, FromLiveId)]
pub struct PanelId(pub LiveId);
*/
struct Drag {
    panel_id: LiveId,
    position: DragPosition,
}

#[derive(Clone, WidgetAction)]
pub enum DockAction {
    SplitPanelChanged {panel_id: LiveId, axis: Axis, align: SplitterAlign},
    TabBarReceivedDraggedItem(LiveId, DraggedItem),
    TabWasPressed(LiveId),
    TabCloseWasPressed(LiveId),
    TabReceivedDraggedItem(LiveId, DraggedItem),
    ContentsReceivedDraggedItem(LiveId, DragPosition, DraggedItem),
    None
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DragPosition {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

impl DragPosition {
    fn compute_drag_position(rect: Rect, position: DVec2) -> DragPosition {
        let top_left = rect.pos;
        let bottom_right = rect.pos + rect.size;
        if (position.x - top_left.x) / rect.size.x < 0.1 {
            DragPosition::Left
        } else if (bottom_right.x - position.x) / rect.size.x < 0.1 {
            DragPosition::Right
        } else if (position.y - top_left.y) / rect.size.y < 0.1 {
            DragPosition::Top
        } else if (bottom_right.y - position.y) / rect.size.y < 0.1 {
            DragPosition::Bottom
        } else {
            DragPosition::Center
        }
    }
    
    fn compute_drag_rect(&self, rect: Rect) -> Rect {
        match self {
            DragPosition::Left => Rect {
                pos: rect.pos,
                size: DVec2 {
                    x: rect.size.x / 2.0,
                    y: rect.size.y,
                },
            },
            DragPosition::Right => Rect {
                pos: DVec2 {
                    x: rect.pos.x + rect.size.x / 2.0,
                    y: rect.pos.y,
                },
                size: DVec2 {
                    x: rect.size.x / 2.0,
                    y: rect.size.y,
                },
            },
            DragPosition::Top => Rect {
                pos: rect.pos,
                size: DVec2 {
                    x: rect.size.x,
                    y: rect.size.y / 2.0,
                },
            },
            DragPosition::Bottom => Rect {
                pos: DVec2 {
                    x: rect.pos.x,
                    y: rect.pos.y + rect.size.y / 2.0,
                },
                size: DVec2 {
                    x: rect.size.x,
                    y: rect.size.y / 2.0,
                },
            },
            DragPosition::Center => rect,
        }
    }
}


#[derive(Clone, PartialEq, WidgetRef)]
pub struct DockRef(WidgetRef);

impl DockRef {
    pub fn close_tab(&self, cx: &mut Cx, tab_id: LiveId) {
        if let Some(mut dock) = self.borrow_mut() {
            dock.close_tab(cx, tab_id);
        }
    }
    
    
    pub fn clicked_tab_close(&self, actions: &WidgetActions) -> Option<LiveId> {
        if let Some(item) = actions.find_single_action(self.widget_uid()) {
            if let DockAction::TabCloseWasPressed(tab_id) = item.action() {
                return Some(tab_id)
            }
        }
        None
    }
}

#[derive(Clone, WidgetSet)]
pub struct DockSet(WidgetSet);

