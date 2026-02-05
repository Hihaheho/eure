use eure::FromEure;

#[derive(Debug, Clone, PartialEq)]
enum EaseFunction {
    Linear,
}

#[derive(Debug, Clone, PartialEq, FromEure)]
#[eure(proxy = "EaseFunction")]
enum EaseFunctionProxy {
    Linear,
}

#[derive(Debug, Clone, PartialEq, FromEure)]
enum LayoutAnimation {
    Curve {
        #[eure(via = "EaseFunctionProxy")]
        ease: EaseFunction,
        duration: f32,
    },
    Immediate,
}

fn main() {}
