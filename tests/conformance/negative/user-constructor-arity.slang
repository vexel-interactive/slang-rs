struct S { int a, b; __init(int x) { a=x; b=x+1; } }; [shader("compute")][numthreads(1,1,1)]void main(){S v=S(1,2);}
