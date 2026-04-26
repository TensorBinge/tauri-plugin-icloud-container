// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "tauri-plugin-icloud-container",
    platforms: [
        .iOS(.v14),
        .macOS(.v11),
    ],
    products: [
        .library(
            name: "tauri-plugin-icloud-container",
            targets: ["ICloudContainer"]
        ),
    ],
    dependencies: [],
    targets: [
        .target(
            name: "ICloudContainer",
            dependencies: [],
            path: "Sources",
            sources: [
                "ICloudContainerPlugin.swift",
                "ICloudContainerResolver.swift",
                "ICloudContainerTauriPlugin.swift",
            ]
        ),
        .testTarget(
            name: "ICloudContainerTests",
            dependencies: ["ICloudContainer"],
            path: "Tests"
        ),
    ]
)
