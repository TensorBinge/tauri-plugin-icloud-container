import XCTest
@testable import ICloudContainer

final class ICloudContainerTests: XCTestCase {
    func testNormalizeIdentifierTrimsWhitespace() {
        XCTAssertEqual(
            ICloudContainerResolver.normalizeIdentifier("  iCloud.com.example.app  "),
            "iCloud.com.example.app"
        )
    }

    func testNormalizeIdentifierReturnsNilForBlankValues() {
        XCTAssertNil(ICloudContainerResolver.normalizeIdentifier(nil))
        XCTAssertNil(ICloudContainerResolver.normalizeIdentifier(""))
        XCTAssertNil(ICloudContainerResolver.normalizeIdentifier("   \n  "))
    }

    func testFreshResolverHasNoCachedContainerUrl() {
        let resolver = ICloudContainerResolver()

        XCTAssertNil(resolver.getCachedContainerUrl(identifier: "iCloud.com.example.app"))
    }

    func testRelativePathStringUsesCanonicalPathComponents() {
        let rootUrl = URL(fileURLWithPath: "/private/var/mobile/Library/Mobile Documents/iCloud~com~tensorbinge~markscope/Documents")
        let itemUrl = URL(fileURLWithPath: "/var/mobile/Library/Mobile Documents/iCloud~com~tensorbinge~markscope/Documents/DndUntitled")

        XCTAssertEqual(relativePathString(from: itemUrl, rootUrl: rootUrl), "DndUntitled")
    }
}
