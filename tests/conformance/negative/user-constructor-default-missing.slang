struct S { int a; __init(int x) { a=x; } }; [shader("compute")][numthreads(1,1,1)]void main(){S v={};}
