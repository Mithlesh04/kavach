// Text: build 1 million words into one text, then split and count them.
var parts = new System.Collections.Generic.List<string>();
for (long i = 0; i < 1000000; i++)
{
    parts.Add("word" + (i % 50));
}
var text = new System.Text.StringBuilder();
for (int i = 0; i < parts.Count; i++)
{
    if (i > 0) text.Append(' ');
    text.Append(parts[i]);
}
string[] words = text.ToString().Split(' ');
long longWords = 0;
foreach (string w in words)
{
    if (w.Length > 5) longWords++;
}
System.Console.WriteLine(longWords);
