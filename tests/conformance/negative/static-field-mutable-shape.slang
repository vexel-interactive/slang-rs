struct S { static int Count = 3; };
[shader("compute")][numthreads(1, 1, 1)]void main() { int values[S.Count]; }
