#if __LINE__ != 1
#error Incorrect initial source line
#endif
#define ID(value) value
#if ID(__LINE__) != 5
#error Incorrect initiating macro line
#endif
#if !defined(__FILE__) || !defined(__SLANG__) || __SLANG__ != 1
#error Missing predefined macros
#endif

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
