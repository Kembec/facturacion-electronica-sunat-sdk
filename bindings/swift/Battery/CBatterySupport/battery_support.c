#include "battery_support.h"

#include <arpa/inet.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

#include <libxml/c14n.h>
#include <libxml/parser.h>

char *battery_c14n(const char *xml, size_t length) {
    if (xml == NULL || length > 0x7fffffff) return NULL;
    xmlDocPtr doc = xmlReadMemory(xml, (int)length, NULL, NULL, XML_PARSE_NONET);
    if (doc == NULL) return NULL;
    xmlChar *out = NULL;
    int written = xmlC14NDocDumpMemory(doc, NULL, XML_C14N_1_0, NULL, 0, &out);
    xmlFreeDoc(doc);
    if (written < 0 || out == NULL) return NULL;
    char *copy = malloc((size_t)written + 1);
    if (copy != NULL) {
        memcpy(copy, out, (size_t)written);
        copy[written] = '\0';
    }
    xmlFree(out);
    return copy;
}

void battery_free(char *text) { free(text); }

#define HELD_MAX 64

static int listener = -1;
static int listener_port;
static volatile int stopping;
static int held[HELD_MAX];
static int held_count;
static pthread_t acceptor;
static pthread_mutex_t gate = PTHREAD_MUTEX_INITIALIZER;

static void *accept_loop(void *unused) {
    (void)unused;
    for (;;) {
        int connection = accept(listener, NULL, NULL);
        if (connection < 0) return NULL;
        if (stopping) { /* the wake-up connection of battery_silent_listener_stop */
            close(connection);
            return NULL;
        }
        pthread_mutex_lock(&gate);
        if (held_count < HELD_MAX) {
            held[held_count++] = connection;
        } else {
            close(connection);
        }
        pthread_mutex_unlock(&gate);
    }
}

int battery_silent_listener_start(void) {
    if (listener >= 0) return -1;
    stopping = 0;
    listener = socket(AF_INET, SOCK_STREAM, 0);
    if (listener < 0) return -1;
    struct sockaddr_in address;
    memset(&address, 0, sizeof address);
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    address.sin_port = 0;
    socklen_t size = sizeof address;
    if (bind(listener, (struct sockaddr *)&address, size) != 0 || listen(listener, 50) != 0 ||
        getsockname(listener, (struct sockaddr *)&address, &size) != 0 ||
        pthread_create(&acceptor, NULL, accept_loop, NULL) != 0) {
        close(listener);
        listener = -1;
        return -1;
    }
    listener_port = ntohs(address.sin_port);
    return listener_port;
}

void battery_silent_listener_stop(void) {
    if (listener < 0) return;
    /* shutdown() does not wake a blocked accept() on macOS: a connection of our own does, and the flag ends the loop. */
    stopping = 1;
    int wake = socket(AF_INET, SOCK_STREAM, 0);
    struct sockaddr_in address;
    memset(&address, 0, sizeof address);
    address.sin_family = AF_INET;
    address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    address.sin_port = htons((unsigned short)listener_port);
    if (wake >= 0) connect(wake, (struct sockaddr *)&address, sizeof address);
    pthread_join(acceptor, NULL);
    if (wake >= 0) close(wake);
    close(listener);
    listener = -1;
    pthread_mutex_lock(&gate);
    for (int i = 0; i < held_count; i++) close(held[i]);
    held_count = 0;
    pthread_mutex_unlock(&gate);
}
