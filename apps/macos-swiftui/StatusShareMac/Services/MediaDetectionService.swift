import Foundation

struct DetectedMedia {
    var title: String = ""
    var artist: String = ""
    var thumbnail: String = ""
}

@MainActor
final class MediaDetectionService {

    /// Single JXA script that handles all media detection:
    /// - Spotify: uses its AppleScript dictionary for title, artist, artworkUrl
    /// - Other apps: uses MRNowPlayingRequest (via osascript entitlements)
    ///
    /// Reference: https://github.com/nohackjustnoobb/media-remote
    private static let jxaScript = """
    function run() {
        ObjC.import('Foundation');

        var bundle = $.NSBundle.bundleWithPath(
            '/System/Library/PrivateFrameworks/MediaRemote.framework/'
        );
        bundle.load;

        var MRNowPlayingRequest = $.NSClassFromString('MRNowPlayingRequest');
        if (!MRNowPlayingRequest) return JSON.stringify(null);

        var isPlaying = MRNowPlayingRequest.localIsPlaying;
        if (!isPlaying) return JSON.stringify(null);

        // Get bundle identifier of the current player
        var bundleId = '';
        try {
            var playerPath = MRNowPlayingRequest.localNowPlayingPlayerPath;
            if (playerPath && playerPath.client) {
                bundleId = ObjC.unwrap(playerPath.client.bundleIdentifier) || '';
                if (!bundleId) {
                    bundleId = ObjC.unwrap(playerPath.client.parentApplicationBundleIdentifier) || '';
                }
            }
        } catch(e) {}

        // Spotify: use its AppleScript dictionary for full info including artwork URL
        if (bundleId === 'com.spotify.client') {
            try {
                var spotify = Application('Spotify');
                if (spotify.running()) {
                    var state = spotify.playerState();
                    if (state === 'playing' || state === 'paused') {
                        var track = spotify.currentTrack;
                        var title = track.name() || '';
                        var artist = track.artist() || '';
                        if (title || artist) {
                            return JSON.stringify({
                                title: title,
                                artist: artist,
                                thumbnail: track.artworkUrl() || ''
                            });
                        }
                    }
                }
            } catch(e) {}
            // Fall through to generic path if Spotify AppleScript fails
        }

        // Generic path: MRNowPlayingRequest
        var item = MRNowPlayingRequest.localNowPlayingItem;
        if (!item) return JSON.stringify(null);

        var infoDict = item.nowPlayingInfo;
        if (!infoDict) return JSON.stringify(null);

        var title = '';
        var artist = '';
        try {
            var v = infoDict.objectForKey('kMRMediaRemoteNowPlayingInfoTitle');
            if (v && !v.isNil()) title = ObjC.unwrap(v);
        } catch(e) {}
        try {
            var v = infoDict.objectForKey('kMRMediaRemoteNowPlayingInfoArtist');
            if (v && !v.isNil()) artist = ObjC.unwrap(v);
        } catch(e) {}

        if (!title && !artist) return JSON.stringify(null);

        return JSON.stringify({
            title: title,
            artist: artist,
            thumbnail: ''
        });
    }
    """

    func detectMedia() async -> DetectedMedia? {
        let script = Self.jxaScript
        let result: String? = await Task.detached {
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
            process.arguments = ["-l", "JavaScript"]

            let stdinPipe = Pipe()
            let stdoutPipe = Pipe()
            let stderrPipe = Pipe()

            process.standardInput = stdinPipe
            process.standardOutput = stdoutPipe
            process.standardError = stderrPipe

            do {
                try process.run()
            } catch {
                return nil
            }

            let scriptData = script.data(using: .utf8)!
            stdinPipe.fileHandleForWriting.write(scriptData)
            stdinPipe.fileHandleForWriting.closeFile()

            process.waitUntilExit()

            guard process.terminationStatus == 0 else { return nil }

            let data = stdoutPipe.fileHandleForReading.readDataToEndOfFile()
            return String(data: data, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines)
        }.value

        guard let result, result != "null", !result.isEmpty else {
            return nil
        }

        guard let jsonData = result.data(using: .utf8),
              let parsed = try? JSONSerialization.jsonObject(with: jsonData) as? [String: Any] else {
            return nil
        }

        let title = parsed["title"] as? String ?? ""
        let artist = parsed["artist"] as? String ?? ""
        let thumbnail = parsed["thumbnail"] as? String ?? ""

        if title.isEmpty && artist.isEmpty {
            return nil
        }

        return DetectedMedia(title: title, artist: artist, thumbnail: thumbnail)
    }
}
