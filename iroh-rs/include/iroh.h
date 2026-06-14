#ifndef IROH_H
#define IROH_H

#include <stdint.h>
#include <stdbool.h>

int64_t iroh_endpoint_new();
bool iroh_endpoint_free(int64_t handle);
int64_t iroh_connect(int64_t endpoint, const char* node_id);
bool iroh_connection_close(int64_t conn);

char* iroh_endpoint_node_id(int64_t endpoint);

#endif