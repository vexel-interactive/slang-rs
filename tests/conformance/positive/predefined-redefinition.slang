#define __LINE__ 7
#if __LINE__ != 7
#error Builtin redefinition did not take effect
#endif
#undef __LINE__
#ifdef __LINE__
#error Undef did not remove the builtin
#endif

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
