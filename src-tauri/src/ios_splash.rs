//! iOS 原生启动 Logo：在 WebView 下面放一个与 LaunchScreen 完全相同的 Logo，
//! 避免「原生启动屏 → WebView 加载中（只有紫色）→ 网页启动画面」之间 Logo 消失又出现。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::NSString;
use objc2_ui_kit::{UIColor, UIImage, UIImageView, UIScrollView, UIView, UIViewAutoresizing};

const LOGO_TAG: isize = 0x616e6b; // "ank"
const LOGO_SIZE: f64 = 120.0;

/// `webview` 是 WKWebView 指针（tauri 的 `PlatformWebview::inner()`）。必须在主线程调用。
pub unsafe fn install_launch_logo(webview: *mut std::ffi::c_void) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let Some(webview) = (webview as *mut UIView).as_ref() else { return };
    let Some(parent) = webview.superview() else { return };
    if parent.viewWithTag(LOGO_TAG).is_some() {
        return;
    }
    let Some(image) = UIImage::imageNamed(&NSString::from_str("LaunchLogo")) else { return };

    // WebView 先透明：页面画出来之前，透出下面的紫色背景和 Logo
    webview.setBackgroundColor(Some(&UIColor::clearColor()));

    let logo: Retained<UIImageView> = UIImageView::initWithImage(mtm.alloc(), Some(&image));
    logo.setFrame(CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(LOGO_SIZE, LOGO_SIZE)));
    let b = parent.bounds();
    logo.setCenter(CGPoint::new(b.origin.x + b.size.width / 2.0, b.origin.y + b.size.height / 2.0));
    logo.setAutoresizingMask(
        UIViewAutoresizing::FlexibleLeftMargin
            | UIViewAutoresizing::FlexibleRightMargin
            | UIViewAutoresizing::FlexibleTopMargin
            | UIViewAutoresizing::FlexibleBottomMargin,
    );
    logo.setTag(LOGO_TAG);
    parent.insertSubview_belowSubview(&logo, webview);
}

/// 网页启动画面淡出后调用：移除原生 Logo，并关闭整页的橡皮筋回弹（各页面自己有滚动区域）
pub unsafe fn remove_launch_logo(webview: *mut std::ffi::c_void) {
    let Some(webview) = (webview as *mut UIView).as_ref() else { return };
    if let Some(parent) = webview.superview() {
        if let Some(logo) = parent.viewWithTag(LOGO_TAG) {
            logo.removeFromSuperview();
        }
    }
    let scroll: Option<Retained<UIScrollView>> = objc2::msg_send![webview, scrollView];
    if let Some(scroll) = scroll {
        scroll.setBounces(false);
    }
}
