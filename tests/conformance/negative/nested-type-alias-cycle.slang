struct Outer { typedef Second First; typedef First Second; };
[shader("compute")][numthreads(1,1,1)]void main() {}
