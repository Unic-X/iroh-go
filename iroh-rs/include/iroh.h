#ifndef IROH_H
#define IROH_H

#include <stdint.h>
#include <stdbool.h>

#include "error.h"

/* Opaque handle types for type safety and clarity */
typedef void* IrohEndpoint;
typedef void* IrohConnection;
typedef void* IrohBuilder;

IrohBuilder iroh_builder_new(void);
void iroh_builder_apply_n0(IrohBuilder builder);
void iroh_builder_apply_minimal(IrohBuilder builder);
bool iroh_builder_bind_addr(IrohBuilder builder, const char* addr);
void iroh_builder_free(IrohBuilder builder);
bool iroh_builder_add_alpn(IrohBuilder builder, const char* alpn);
void iroh_builder_relay_mode(IrohBuilder builder, uint8_t mode);
bool iroh_builder_secret_key(IrohBuilder builder, const uint8_t* key);

// IrohEndpoint iroh_endpoint_new(void);
// void iroh_endpoint_free(IrohEndpoint handle);

IrohConnection iroh_endpoint_connect(IrohEndpoint endpoint, const char* endpoint_id);
void iroh_connection_free(IrohConnection conn);



#endif   