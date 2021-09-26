initSidebarItems({"enum":[["ColorEditDisplayMode","Color editor display mode."],["ColorEditInputMode","Color editor input mode."],["ColorFormat","Color component formatting"],["ColorPickerMode","Color picker hue/saturation/value editor mode"],["ColorPreview","Color editor preview style"],["ComboBoxHeight","Combo box height mode."],["ComboBoxPreviewMode","Combo box preview mode."],["Condition","Condition for applying a setting"],["Direction","A cardinal direction"],["DrawCmd","A draw command"],["EditableColor","Mutable reference to an editable color value."],["FocusedWidget","Target widget selection for keyboard focus"],["FontAtlasRef","An immutably borrowed reference to a (possibly shared) font atlas"],["FontAtlasRefMut","A mutably borrowed reference to a (possibly shared) font atlas"],["FontSource","A source for binary font data"],["HistoryDirection","The arrow key a user pressed to trigger the `on_history` callback."],["Id","Unique ID used by widgets"],["ItemFlag","A temporary change in item flags"],["Key","A key identifier"],["MouseButton","Represents one of the supported mouse buttons"],["MouseCursor","Mouse cursor type identifier"],["NavInput","An input identifier for navigation"],["StyleColor","A color identifier for styling"],["StyleVar","A temporary change in user interface style"],["TreeNodeId","Unique ID used by tree nodes"]],"fn":[["dear_imgui_version","Returns the underlying Dear ImGui library version"]],"macro":[["create_token","This is a macro used internally by imgui-rs to create StackTokens representing various global state in DearImGui."],["im_str",""]],"mod":[["__core","The Rust Core Library"],["color",""],["drag_drop","Structs to create a Drag and Drop sequence. Almost all structs are re-exported and can be accessed from the crate root; some additional utilities can be found in here."],["draw_list","The draw list lets you create custom graphics within a window."],["internal","Internal raw utilities (don’t use unless you know what you’re doing!)"],["sys",""]],"struct":[["AngleSlider","Builder for an angle slider widget."],["BackendFlags","Backend capabilities"],["ButtonFlags","Flags for invisible buttons"],["ChannelsSplit","Represent the drawing interface within a call to `channels_split`."],["ChildWindow","Builder for a child window"],["ChildWindowToken","Tracks a child window that can be ended by calling `.end()` or by dropping"],["CollapsingHeader","Builder for a collapsing header widget"],["ColorButton","Builder for a color button widget."],["ColorEdit","Builder for a color editor widget."],["ColorEditFlags","Color edit flags"],["ColorPicker","Builder for a color picker widget."],["ColorStackToken","Tracks a color pushed to the color stack that can be popped by calling `.end()` or by dropping."],["ComboBox","Builder for a combo box widget"],["ComboBoxFlags","Flags for combo boxes"],["ComboBoxToken","Tracks a combo box that can be ended by calling `.end()` or by dropping."],["ConfigFlags","Configuration flags"],["Context","An imgui-rs context."],["DisabledToken","Starts a scope where interaction is disabled. Ends be calling `.end()` or when the token is dropped."],["Drag","Builder for a drag slider widget."],["DragDropFlags","Flags for igBeginDragDropSource(), igAcceptDragDropPayload()"],["DragDropSource","Creates a source for drag drop data out of the last ID created."],["DragDropTarget","Creates a target for drag drop data out of the last ID created."],["DragRange","Builder for a drag slider widget."],["DrawCmdIterator",""],["DrawCmdParams",""],["DrawData","All draw data to render a Dear ImGui frame."],["DrawList","Draw command list"],["DrawListIterator","Iterator over draw lists"],["DrawListMut","Object implementing the custom draw API."],["DrawVert","A single vertex"],["DummyClipboardContext",""],["Font","Runtime data for a single font within a font atlas"],["FontAtlas","A font atlas that builds a single texture"],["FontAtlasFlags","Font atlas configuration flags"],["FontAtlasTexture","Handle to a font atlas texture"],["FontConfig","Configuration settings for a font"],["FontGlyph","A single font glyph"],["FontGlyphRanges","A set of Unicode codepoints"],["FontId","A font identifier"],["FontStackToken","Tracks a font pushed to the font stack that can be popped by calling `.end()` or by dropping."],["GroupToken","Tracks a layout group that can be ended with `end` or by dropping."],["IdStackToken","Tracks an ID pushed to the ID stack that can be popped by calling `.pop()` or by dropping."],["ImColor32","Wraps u32 that represents a packed RGBA color. Mostly used by types in the low level custom drawing API, such as `DrawListMut`."],["ImStr","A UTF-8 encoded, implicitly nul-terminated string slice."],["ImString","A UTF-8 encoded, growable, implicitly nul-terminated string."],["Image","Builder for an image widget"],["ImageButton","Builder for an image button widget"],["InputFloat",""],["InputFloat2",""],["InputFloat3",""],["InputFloat4",""],["InputInt",""],["InputInt2",""],["InputInt3",""],["InputInt4",""],["InputText",""],["InputTextCallback","Callback flags for an `InputText` widget. These correspond to the general textflags."],["InputTextFlags","Flags for text inputs"],["InputTextMultiline",""],["InputTextMultilineCallback","Callback flags for an `InputTextMultiline` widget. These correspond to the general textflags."],["Io","Settings and inputs/outputs for imgui-rs"],["ItemFlagsStackToken","Tracks a change pushed to the item flags stack"],["ItemHoveredFlags","Item hover check option flags"],["ItemWidthStackToken",""],["ListBox","Builder for a list box widget"],["ListBoxToken","Tracks a list box that can be ended by calling `.end()` or by dropping"],["ListClipper",""],["MainMenuBarToken","Tracks a main menu bar that can be ended by calling `.end()` or by dropping"],["MenuBarToken","Tracks a menu bar that can be ended by calling `.end()` or by dropping"],["MenuItem","Builder for a menu item."],["MenuToken","Tracks a menu that can be ended by calling `.end()` or by dropping"],["MultiColorStackToken","Tracks one or more changes pushed to the color stack that must be popped by calling `.pop()`"],["MultiStyleStackToken","Tracks one or more changes pushed to the style stack that must be popped by calling `.pop()`"],["PassthroughCallback","This is a Zst which implements TextCallbackHandler as a passthrough."],["PlotHistogram",""],["PlotLines",""],["PopupModal","Create a modal pop-up."],["PopupToken","Tracks a popup token that can be ended with `end` or by dropping."],["ProgressBar","Builder for a progress bar widget."],["Selectable","Builder for a selectable widget."],["SelectableFlags","Flags for selectables"],["SharedFontAtlas","A font atlas that can be shared between contexts"],["Slider","Builder for a slider widget."],["SliderFlags","Flags for sliders"],["Style","User interface style/colors"],["StyleStackToken","Tracks a style pushed to the style stack that can be popped by calling `.end()` or by dropping."],["SuspendedContext","A suspended imgui-rs context."],["TabBar","Builder for a tab bar."],["TabBarFlags",""],["TabBarToken","Tracks a window that can be ended by calling `.end()` or by dropping"],["TabItem",""],["TabItemFlags",""],["TabItemToken","Tracks a tab bar item that can be ended by calling `.end()` or by dropping"],["TextCallbackData","This struct provides methods to edit the underlying text buffer that Dear ImGui manipulates. Primarily, it gives remove_chars, insert_chars, and mutable access to what text is selected."],["TextWrapPosStackToken","Tracks a change pushed to the text wrap position stack"],["TextureId","An opaque texture identifier"],["Textures","Generic texture mapping for use by renderers."],["TooltipToken","Tracks a layout tooltip that can be ended by calling `.end()` or by dropping."],["TreeNode","Builder for a tree node widget"],["TreeNodeFlags","Flags for tree nodes"],["TreeNodeToken","Tracks a tree node that can be popped by calling `.pop()`, `end()`, or by dropping."],["Ui","A temporary reference for building the user interface for one frame"],["VerticalSlider","Builder for a vertical slider widget."],["Window","Builder for a window"],["WindowFlags","Configuration flags for windows"],["WindowFocusedFlags","Window focus check option flags"],["WindowHoveredFlags","Window hover check option flags"],["WindowToken","Tracks a window that can be ended by calling `.end()` or by dropping."]],"trait":[["ClipboardBackend","Trait for clipboard backends"],["InputTextCallbackHandler","This trait provides an interface which ImGui will call on `InputText` and `InputTextMultiline` callbacks."]],"type":[["DrawIdx","A vertex index"]]});