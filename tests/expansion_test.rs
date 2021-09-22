use error_chain_utils::error_chain_quick;
#[allow(unused_imports)]
use error_chain::error_chain;

#[test]
fn expand_macro() {
    error_chain_quick!{
        types {
            BuildError, BEKind, BETrait, BEResult;
        }
    
        errors {
            NormalError1 {
                description("Error 1 Description: Without Arguments"),
                display("Error 1 Display")
            }
            NormalError2 (arg1: String, arg2: String) {
                description("Error 2 Description: With Arguments"),
                display("Error 2 Display: {}, {}", arg1, arg2),
            }
            quick!(QuickError1, "Error 1 Description: Zero arguments")
            quick!(QuickError2, "Error 2 Description: One Argument",(arg1,))
            quick!(QuickError3, "Error 3 Description: Three Arguments",(arg1,arg2,arg3,))
        }
    }
}