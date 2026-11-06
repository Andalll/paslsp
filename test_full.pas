program test;

var
    x, y: integer;
    msg: string;

procedure add(a, b: integer);
begin
    x := a + b;
end;

function double(n: integer): integer;
begin
    double := n * 2;
end;

begin
    x := 10;
    y := 20;
    add(x, y);
    msg := 'hello';
end.
