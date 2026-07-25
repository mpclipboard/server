import Cocoa
import UserNotifications

final class AppDelegate: NSObject, NSApplicationDelegate {
    private let mpclipboard: MPClipboard = MPClipboard()
    private var mpclipboardSource: DispatchSourceRead?

    private let clipboard: Clipboard = Clipboard()
    private var clipboardTimer: Timer?

    private let tray: Tray = Tray()

    func applicationDidFinishLaunching(_ aNotification: Notification) {
        ProcessInfo.processInfo.disableAutomaticTermination("MPClipboard runs continuously as a menu bar clipboard sync agent")
        ProcessInfo.processInfo.disableSuddenTermination()
        // Hide Dock icon
        NSApp.setActivationPolicy(.accessory)

        UNUserNotificationCenter.current().requestAuthorization(options: [.alert]) { granted, error in
            if granted {
                print("Got permission to send notifications")
            } else {
                fputs("Failed to get permission to send notifications", stderr)
                if let error = error {
                    fputs("Error showing notification: \(error)", stderr)
                }
            }
        }

        let source = DispatchSource.makeReadSource(fileDescriptor: mpclipboard.fd(), queue: .global())
        source.setEventHandler { [weak self] in
            self?.readMPClipboard()
        }
        source.resume()
        mpclipboardSource = source

        clipboardTimer = clipboard.startPolling(onCopy: { text in
            switch self.mpclipboard.pushText(text) {
            case .droppedAsStale:
                return
            case .error:
                fatalError("MPClipboard return error from .push_text()")
            case .sent:
                self.tray.pushSent(text)
            }
        })
    }

    @objc
    func quit() {
        print("Quitting...")
        self.clipboardTimer?.invalidate()
        NSApp.terminate(self)
    }

    private func readMPClipboard() {
        guard let output = mpclipboard.read() else {
            return
        }

        DispatchQueue.main.async {
            switch output {
            case .connectivityChanged(let connectivity):
                self.tray.setConnectivity(connectivity)
            case .newText(let text):
                self.clipboard.writeText(text)
                self.tray.pushReceived(text)
                self.showNotification(text)
            case .error:
                fatalError("MPClipboard return error from .read()")
            }
        }
    }

    private func showNotification(_ text: String) {
        let content = UNMutableNotificationContent()
        content.title = "MPClipboard"
        content.body = text

        let request = UNNotificationRequest(identifier: UUID().uuidString, content: content, trigger: nil)
        UNUserNotificationCenter.current().add(request) { error in
            if let error = error {
                fputs("Error showing notification: \(error)", stderr)
            }
        }
    }
}
