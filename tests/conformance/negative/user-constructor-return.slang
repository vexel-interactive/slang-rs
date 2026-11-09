struct S { int a; __init(int x) { return x; } }; [shader("compute")][numthreads(1,1,1)]void main(){S v=S(2);}
