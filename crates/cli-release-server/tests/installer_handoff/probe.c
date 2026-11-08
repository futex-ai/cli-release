/* Native installer handoff probe with an explicit readiness handshake. */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

int main(int argc, char **argv) {
    const char *record_path = getenv("HANDOFF_ARGUMENTS");
    const char *status_text = getenv("HANDOFF_STATUS");
    if (record_path == NULL || status_text == NULL) {
        return 80;
    }
    int status = atoi(status_text);
    FILE *record = fopen(record_path, "wb");
    if (record == NULL) {
        return 81;
    }
    for (int index = 1; index < argc; ++index) {
        size_t length = strlen(argv[index]) + 1;
        if (fwrite(argv[index], 1, length, record) != length) {
            fclose(record);
            return 82;
        }
    }
    if (fclose(record) != 0) {
        return 83;
    }
    for (int index = 0; index < argc; ++index) {
        memset(argv[index], 0, strlen(argv[index]));
    }
    printf("%ld\n", (long)getpid());
    if (fflush(stdout) != 0 || getchar() != 'x') {
        return 84;
    }
    return status;
}
