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
            type: .static,
            targets: ["ICloudContainer"]
        ),
    ],
    dependencies: [
        .package(name: "Tauri", path: "../.tauri/tauri-api")
    ],
    targets: [
        .target(
            name: "ICloudContainer",
            dependencies: [
                .product(name: "Tauri", package: "Tauri")
            ],
            path: "Sources",
            sources: [
                "ICloudContainerDTOs.swift",
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
