#import <Foundation/Foundation.h>

NS_ASSUME_NONNULL_BEGIN

/// The checks of `diagnosis` on the four SOAP results, each reported through `check`. Not a public header.
void KSSCheckDiagnosis(NSURL *repository, void (^check)(BOOL ok, NSString *what));

NS_ASSUME_NONNULL_END
