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

enum PushResult {
    case sent
    case droppedAsStale
    case error

    static func from(_ pushResult: mpclipboard_PushResult) -> Self {
        switch pushResult {
        case MPCLIPBOARD_PUSH_RESULT_SENT:
            .sent
        case MPCLIPBOARD_PUSH_RESULT_DROPPED_AS_STALE:
            .droppedAsStale
        case MPCLIPBOARD_PUSH_RESULT_ERROR:
            .error
        default:
            fatalError("unsupported PushResult")
        }
    }
}

enum Output {
    case connectivityChanged(Connectivity)
    case newText(String)
    case error

    static func from(_ output: mpclipboard_Output) -> Self? {
        switch output.tag {
        case MPCLIPBOARD_OUTPUT_CONNECTIVITY_CHANGED:
            return .connectivityChanged(Connectivity.from(output.CONNECTIVITY_CHANGED.connectivity))
        case MPCLIPBOARD_OUTPUT_NEW_TEXT:
            let (ptr, len) = (output.NEW_TEXT.ptr!, output.NEW_TEXT.len)
            let data = Data(bytes: ptr, count: len)
            free(ptr)

            if let text = String(data: data, encoding: .utf8) {
                return .newText(text)
            } else {
                fatalError("non-utf8 new text in output")
            }
        case MPCLIPBOARD_OUTPUT_IGNORE:
            return nil
        case MPCLIPBOARD_OUTPUT_ERROR:
            return .error
        default:
            fatalError("unsupported Output")
        }
    }
}

final class MPClipboard {
    private let handle: OpaquePointer

    init() {
        mpclipboard_init()

#if DEBUG
        puts("Debug build, using local config")
        var option = MPCLIPBOARD_CONFIG_READ_OPTION_FROM_LOCAL_FILE
#else
        puts("Release build, using config from XDG dir")
        var option = MPCLIPBOARD_CONFIG_READ_OPTION_FROM_XDG_CONFIG_DIR
#endif

        guard let config = mpclipboard_config_read(option) else {
            fatalError("NULL config")
        }

        guard let handle = mpclipboard_new(config) else {
            fatalError("NULL mpclipboard")
        }

        self.handle = handle
    }

    func fd() -> Int32 {
        mpclipboard_get_fd(handle)
    }

    func pushText(_ text: String) -> PushResult {
        text.withCString { ptr in
            PushResult.from(mpclipboard_push_text1(handle, ptr))
        }
    }

    func read() -> Output? {
        Output.from(mpclipboard_read(handle))
    }
}
