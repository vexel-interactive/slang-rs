module predefined;

#if !__SLANG__ || !defined(__LINE__) || !defined(__FILE__)
#error Missing module source macros
#endif

public int sourceLine() { return __LINE__; }
