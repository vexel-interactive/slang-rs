module macro_lib;
#ifdef ROOT_ONLY
#error root macro leaked
#endif
#define IMPORT_ONLY 8
public float macroRead(){return IMPORT_ONLY;}
