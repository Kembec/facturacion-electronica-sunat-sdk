#import "diagnosis.h"

#import <KembecSunatSDKObjC/KembecSunatSDKObjC.h>

// A consumer has no way to hand a response to the results: they come out of the library's calls, and the SUNAT answer
// that carries a diagnosis cannot be had offline. The battery reads the core's own output through the package's parser
// (KSSResults+Private.h, KSSFields), the way KSSRun does after a call.
#import "../../Sources/KembecSunatSDKObjC/KSSResults+Private.h"

@interface KSSFields (BatteryParsing)
- (instancetype)initWithDictionary:(NSDictionary *)dictionary path:(NSString *)path problems:(NSMutableArray *)problems;
@end

static KSSFields *KSSFieldsOf(NSString *json, NSMutableArray *problems) {
    id object = [NSJSONSerialization JSONObjectWithData:[json dataUsingEncoding:NSUTF8StringEncoding] options:0 error:NULL];
    if (![object isKindOfClass:[NSDictionary class]]) return nil;
    return [[KSSFields alloc] initWithDictionary:object path:@"" problems:problems];
}

/// The core's answers of bindings/swift/Battery/diagnosis (the files the Swift, PHP and wasm bindings check):
/// send-beta-2800 is the beta's real answer to a boleta (17/10/2026).
void KSSCheckDiagnosis(NSURL *repository, void (^check)(BOOL, NSString *)) {
    NSURL *directory = [repository URLByAppendingPathComponent:@"bindings/swift/Battery/diagnosis"];
    NSString *(^read)(NSString *) = ^NSString *(NSString *name) {
        return [NSString stringWithContentsOfURL:[directory URLByAppendingPathComponent:name] encoding:NSUTF8StringEncoding error:NULL] ?: @"";
    };
    NSMutableArray *problems = [NSMutableArray array];

    KSSSendResult *send = [KSSSendResult resultWithFields:KSSFieldsOf(read(@"send-beta-2800.json"), problems)];
    check(send.outcome == KSSOutcomeFault && [send.fault.code isEqualToString:@"2800"] && send.fault.responseClass == KSSResponseClassRejected,
          @"diagnosis: the 2800 is a rejected fault");
    KSSDiagnosis *first = send.diagnosis.firstObject;
    check(send.diagnosis.count == 1 && [first.code isEqualToString:@"2800"], [NSString stringWithFormat:@"diagnosis: %@", send.diagnosis]);
    check([first.detail.node isEqualToString:@"cbc:ID/schemeID"] && [first.detail.value isEqualToString:@"1"],
          [NSString stringWithFormat:@"diagnosis: detail %@ %@", first.detail.node, first.detail.value]);
    KSSDiagnosisField *field = first.fields.firstObject;
    check(first.fields.count == 1 && [field.inputPath isEqualToString:@"customer.document_type"] &&
              field.certainty == KSSDiagnosisCertaintySource && [field.xpath hasSuffix:@"cbc:ID@schemeID"],
          [NSString stringWithFormat:@"diagnosis: fields %@", first.fields]);

    KSSStatusResult *observed = [KSSStatusResult resultWithFields:KSSFieldsOf(read(@"status-observed-4287.json"), problems)];
    NSArray<NSString *> *codes = [observed.diagnosis valueForKey:@"code"];
    check([codes isEqualToArray:@[@"4287", @"4001"]], [NSString stringWithFormat:@"diagnosis: observed notes %@", codes]);
    KSSDiagnosisField *price = observed.diagnosis.firstObject.fields.firstObject;
    check(observed.diagnosis.firstObject.detail == nil && [price.inputPath isEqualToString:@"lines[].unit_price"] &&
              price.certainty == KSSDiagnosisCertaintyInferred,
          @"diagnosis: 4287");
    check(observed.diagnosis.lastObject.fields.count == 0, @"diagnosis: 4001 is not in the 2026 rules");

    NSDictionary<NSString *, NSArray *> *without = @{
        @"send": [KSSSendResult resultWithFields:KSSFieldsOf(@"{\"outcome\":\"fault\",\"code\":\"1033\",\"message\":\"x\",\"class\":\"already_registered\"}", problems)].diagnosis ?: @[@"nil"],
        @"summary": [KSSSummaryResult resultWithFields:KSSFieldsOf(@"{\"outcome\":\"ticket\",\"ticket\":\"1\"}", problems)].diagnosis ?: @[@"nil"],
        @"ticket": [KSSTicketResult resultWithFields:KSSFieldsOf(@"{\"outcome\":\"no_verdict\",\"reason\":\"timeout\",\"message\":\"x\",\"ticket\":\"1\"}", problems)].diagnosis ?: @[@"nil"],
        @"status": [KSSStatusResult resultWithFields:KSSFieldsOf(@"{\"outcome\":\"no_verdict\",\"reason\":\"timeout\",\"message\":\"x\"}", problems)].diagnosis ?: @[@"nil"],
    };
    for (NSString *name in without) {
        check(without[name].count == 0, [NSString stringWithFormat:@"diagnosis: %@ without `diagnosis` is %@", name, without[name]]);
    }
    check(problems.count == 0, [NSString stringWithFormat:@"diagnosis: problems %@", problems]);
}
