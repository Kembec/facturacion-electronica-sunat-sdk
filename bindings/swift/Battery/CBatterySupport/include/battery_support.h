#ifndef BATTERY_SUPPORT_H
#define BATTERY_SUPPORT_H

/* What the Swift and Objective-C batteries need from C, so both check the same way on every platform: inclusive C14N
 * 1.0 from libxml2 (not from the SDK under test) and a TCP listener that accepts and never answers. */

#include <stddef.h>

/* Inclusive C14N 1.0 without comments of `xml` (`length` bytes). Returns a NUL-terminated string to release with
 * battery_free, or NULL if the text is not XML. */
char *battery_c14n(const char *xml, size_t length);
void battery_free(char *text);

/* Starts a listener on 127.0.0.1 that accepts connections and holds them without sending a byte: a TLS handshake
 * against it waits out its timeout. Returns the port, or -1. One at a time. */
int battery_silent_listener_start(void);
void battery_silent_listener_stop(void);

#endif
