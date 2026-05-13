#include <stddef.h>
#include <string.h>
#include "../ocgcore/ocgapi.h"
#include "../ocgcore/ocgapi_types.h"
#include "../ocgcore/ocgapi_constants.h"

extern "C" {

#define MAX_CARD_DB 256

typedef struct {
    uint32_t code;
    OCG_CardData data;
} CardEntry;

static CardEntry g_card_db[MAX_CARD_DB];
static int g_card_db_size = 0;

static void fill_card_data(OCG_CardData* data, uint32_t code, uint32_t type, uint32_t level, uint32_t attribute, uint64_t race, int32_t attack, int32_t defense) {
    memset(data, 0, sizeof(*data));
    data->code = code;
    data->type = type;
    data->level = level;
    data->attribute = attribute;
    data->race = race;
    data->attack = attack;
    data->defense = defense;
}

// Called from Rust to register card data before duel creation
void OCG_RegisterCardData(uint32_t code, uint32_t type, uint32_t level,
                           uint32_t attribute, uint32_t race, int32_t atk, int32_t def) {
    if (g_card_db_size >= MAX_CARD_DB) return;
    CardEntry* e = &g_card_db[g_card_db_size++];
    fill_card_data(&e->data, code, type, level, attribute, (uint64_t)race, atk, def);
    e->code = code;
}

// Stub implementations of required callbacks
void stub_data_reader(void* payload, uint32_t code, OCG_CardData* data) {
    (void)payload;
    if(data == NULL) {
        return;
    }

    // Search registry for registered card data only
    for (int i = 0; i < g_card_db_size; i++) {
        if (g_card_db[i].code == code) {
            *data = g_card_db[i].data;
            return;
        }
    }

    // No fallback - leave data zeroed out if card not found
    // Cards without registered data will fail to load
}

void stub_data_reader_done(void* payload, OCG_CardData* data) {
    (void)payload;
    (void)data;
}

int stub_script_reader(void* payload, OCG_Duel duel, const char* name) {
    (void)payload;
    (void)duel;
    (void)name;
    return 0;
}

void stub_log_handler(void* payload, const char* string, int type) {
    (void)payload;
    (void)string;
    (void)type;
}

// Helper function to create a duel with stub callbacks
int OCG_CreateDuelWithStubs(OCG_Duel* out_ocg_duel, const OCG_DuelOptions* options_ptr) {
    OCG_DuelOptions options = *options_ptr;
    options.cardReader = stub_data_reader;
    options.payload1 = NULL;
    options.scriptReader = stub_script_reader;
    options.payload2 = NULL;
    options.logHandler = stub_log_handler;
    options.payload3 = NULL;
    options.cardReaderDone = stub_data_reader_done;
    options.payload4 = NULL;
    return OCG_CreateDuel(out_ocg_duel, &options);
}

}  // extern "C"

