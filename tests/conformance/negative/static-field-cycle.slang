struct S { static const int first = S.second; static const int second = S.first; };
[shader("compute")][numthreads(1, 1, 1)]void main() {}
