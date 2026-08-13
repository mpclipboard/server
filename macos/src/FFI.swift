import Foundation

enum Connectivity {
    case connecting
    case connected
    case disconnected

    static func from(_ connectivity: mpclipboard_Connectivity) -> Self {
        switch connectivity {
        case MPCLIPBOARD_CONNECTIVITY_CONNECTING:
            .connecting
        case MPCLIPBOARD_CONNECTIVITY_CONNECTED:
            .connected
        case MPCLIPBOARD_CONNECTIVITY_DISCONNECTED:
            .disconnected
        default:
            fatalError("unsupported Connectivity")
        }
    }
}

struct Output {
    let connectivity: Connectivity?
    let text: String?

    static func from(_ output: mpclipboard_Output) -> Self? {
        switch output.tag {
        case MPCLIPBOARD_OUTPUT_CONNECTIVITY_CHANGED:
            return Output(connectivity: Connectivity.from(output.CONNECTIVITY_CHANGED.connectivity), text: nil)
        case MPCLIPBOARD_OUTPUT_NEW_TEXT:
            return Output(connectivity: nil, text: string(ptr: output.NEW_TEXT.ptr, len: output.NEW_TEXT.len))
        case MPCLIPBOARD_OUTPUT_BOTH:
            return Output(
                connectivity: Connectivity.from(output.BOTH.connectivity),
                text: string(ptr: output.BOTH.ptr, len: output.BOTH.len)
            )
        case MPCLIPBOARD_OUTPUT_IGNORE:
            return nil
        case MPCLIPBOARD_OUTPUT_ERROR:
            fatalError("MPClipboard return error from .read()")
        default:
            fatalError("unsupported Output")
        }
    }

    private static func string(ptr: UnsafeMutablePointer<CChar>?, len: Int) -> String {
        let ptr = ptr!
        let data = Data(bytes: ptr, count: len)
        free(ptr)

        guard let text = String(data: data, encoding: .utf8) else {
            fatalError("non-utf8 new text in output")
        }

        return text
    }
}

final class MPClipboard {
    private let handle: OpaquePointer

    init() {
#if DEBUG
        puts("Debug build, using local config")
        guard let handle = mpclipboard_new_with_local_config() else {
            fatalError("NULL mpclipboard")
        }
#else
        puts("Release build, using config from XDG dir")
        guard let handle = mpclipboard_new_with_xdg_config() else {
            fatalError("NULL mpclipboard")
        }
#endif

        self.handle = handle
    }

    deinit {
        mpclipboard_drop(handle)
    }

    func fd() -> Int32 {
        mpclipboard_get_fd(handle)
    }

    func pushText(_ text: String) -> Bool {
        text.utf8CString.withUnsafeBufferPointer { bytes in
            mpclipboard_push_text(handle, bytes.baseAddress, bytes.count - 1)
        }
    }

    func read() -> Output? {
        Output.from(mpclipboard_read(handle))
    }
}
