#ifndef STATUSSHARE_C_API_H
#define STATUSSHARE_C_API_H

#ifdef __cplusplus
extern "C" {
#endif

char *ss_fetch_status(const char *config_json);
char *ss_push_status(const char *config_json, const char *update_json);
char *ss_default_config_file_path(void);
char *ss_default_persisted_config(void);
char *ss_load_persisted_config(const char *path);
char *ss_save_persisted_config(const char *path, const char *config_json);
char *ss_resolve_status_update(const char *matching_json, const char *input_json);
void ss_string_free(char *ptr_value);

#ifdef __cplusplus
}
#endif

#endif

