implementing members;
#ifdef SOURCE_ONLY
#error primary macro leaked
#endif
__include member_b;
float aValue(){return bValue();}
