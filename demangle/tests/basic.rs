extern crate symbolic_demangle;

use symbolic_demangle::{demangle, DemangleOptions, Symbol, Language};


fn assert_mangle(input: &str, output: Option<&str>, opts: DemangleOptions) {
    if let Some(rv) = demangle(input, &opts).unwrap() {
        assert_eq!(Some(rv.as_str()), output);
    } else {
        assert_eq!(None, output);
    }
}


#[test]
fn test_rust_demangle() {
    assert_mangle("__ZN3std2io4Read11read_to_end17hb85a0f6802e14499E", Some("std::io::Read::read_to_end"), Default::default());

    let sym = Symbol::new("__ZN3std2io4Read11read_to_end17hb85a0f6802e14499E");
    assert_eq!(sym.language(), Some(Language::Rust));
    assert_eq!(sym.to_string(), "std::io::Read::read_to_end");

    assert_mangle("__ZN82_$LT$std..sys_common..poison..PoisonError$LT$T$GT$$u20$as$u20$core..fmt..Debug$GT$3fmt17h0853873ca77ac01aE",
                  Some("<std::sys_common::poison::PoisonError<T> as core::fmt::Debug>::fmt"),
                  Default::default());
}

#[test]
fn test_swift_demangle() {
    assert_mangle("_TFC12Swift_Tester14ViewController11doSomethingfS0_FT_T_",
                  Some("ViewController.doSomething"),
                  Default::default());

    assert_mangle("_TFC12Swift_Tester14ViewController11doSomethingfS0_FT_T_",
                  Some("ViewController.doSomething(_:)"),
                  DemangleOptions {
                      with_arguments: true,
                      ..Default::default()
                  });

    assert_mangle("_TTWVSC29UIApplicationLaunchOptionsKeys21_ObjectiveCBridgeable5UIKitZFS0_36_unconditionallyBridgeFromObjectiveCfGSqwx15_ObjectiveCType_x",
                  Some("protocol witness for static _ObjectiveCBridgeable._unconditionallyBridgeFromObjectiveC in conformance UIApplicationLaunchOptionsKey"),
                  Default::default());

    let sym = Symbol::new("_TFC12Swift_Tester14ViewController11doSomethingfS0_FT_T_");
    assert_eq!(sym.language(), Some(Language::Swift));
    assert_eq!(sym.to_string(), "ViewController.doSomething");
}

#[test]
fn test_cpp_demangle() {
    assert_mangle("_Z28JS_GetPropertyDescriptorByIdP9JSContextN2JS6HandleIP8JSObjectEENS2_I4jsidEENS1_13MutableHandleINS1_18PropertyDescriptorEEE",
                  Some("JS_GetPropertyDescriptorById"), DemangleOptions {
                      with_arguments: false,
                      ..Default::default()
                  });
    assert_mangle("_Z28JS_GetPropertyDescriptorByIdP9JSContextN2JS6HandleIP8JSObjectEENS2_I4jsidEENS1_13MutableHandleINS1_18PropertyDescriptorEEE",
                  Some("JS_GetPropertyDescriptorById(JSContext*, JS::Handle<JSObject*>, JS::Handle<jsid>, JS::MutableHandle<JS::PropertyDescriptor>)"), DemangleOptions {
                      with_arguments: true,
                      ..Default::default()
                  });

    let sym = Symbol::new("_Z28JS_GetPropertyDescriptorByIdP9JSContextN2JS6HandleIP8JSObjectEENS2_I4jsidEENS1_13MutableHandleINS1_18PropertyDescriptorEEE");
    assert_eq!(sym.language(), Some(Language::Cpp));
    assert_eq!(sym.to_string(), "JS_GetPropertyDescriptorById");
}

#[test]
fn test_objc_demangle_noop() {
    let sym = Symbol::new("+[KSCrashReportFilterObjectForKey filterWithKey:allowNotFound:]");
    assert_eq!(sym.language(), Some(Language::ObjC));
    assert_eq!(sym.to_string(), "+[KSCrashReportFilterObjectForKey filterWithKey:allowNotFound:]");
}

#[test]
fn test_no_match() {
    assert_mangle("foo", None, Default::default());

    let sym = Symbol::new("bla_bla_bla");
    assert_eq!(sym.language(), None);
    assert_eq!(sym.to_string(), "bla_bla_bla");
}