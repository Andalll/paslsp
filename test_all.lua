vim.wait(5000, function() return #vim.diagnostic.get(0) > 0 end)
local diags = vim.diagnostic.get(0)
print("Diagnostics count: " .. #diags)
for i, d in ipairs(diags) do
  local lsp = d.user_data and d.user_data.lsp
  if lsp then
    print(i .. ". " .. lsp.message .. " (line " .. (lsp.range.start.line + 1) .. ", sev=" .. lsp.severity .. ")")
  end
end

-- Test hover at line 0, character 8 (on "test" in "program test")
vim.lsp.buf_request_sync(0, 'textDocument/hover', {
  textDocument = { uri = vim.uri_from_fname('/data/data/com.termux/files/home/storage/projects/PasLsp/test_error.pas') },
  position = { line = 0, character = 8 },
}, 2000)

-- Test completion
vim.lsp.buf_request_sync(0, 'textDocument/completion', {
  textDocument = { uri = vim.uri_from_fname('/data/data/com.termux/files/home/storage/projects/PasLsp/test_error.pas') },
  position = { line = 0, character = 8 },
}, 2000)

local c = vim.lsp.get_clients({name='paslsp'})[1]
print("LSP: " .. tostring(c and c.name or "nil"))
