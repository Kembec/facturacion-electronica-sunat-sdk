#import <Foundation/Foundation.h>

#import "KSSObjCBattery.h"

// objc-battery <repository>: exit 0 only if every check passed.
int main(int argc, const char *argv[]) {
    @autoreleasepool {
        if (argc != 2) {
            fprintf(stderr, "usage: objc-battery <repository>\n");
            return 2;
        }
        KSSObjCBatteryReport *report = [KSSObjCBattery runWithRepository:[NSURL fileURLWithPath:@(argv[1])]];
        for (NSString *failure in report.failures) fprintf(stderr, "FAIL %s\n", failure.UTF8String);
        if (report.failures.count > 0) {
            fprintf(stderr, "%s\n", report.summary.UTF8String);
            return 1;
        }
        printf("%s\n", report.summary.UTF8String);
        return 0;
    }
}
