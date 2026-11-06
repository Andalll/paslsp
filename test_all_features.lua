local uri = vim.uri_from_fname('/data/data/com.termux/files/home/storage/projects/PasLsp/test_full.pas')
local f = io.open('/data/data/com.termux/files/home/storage/projects/PasLsp/lsp_test_result.txt', 'w')

local function req(method, params)
  local r = vim.lsp.buf_request_sync(0, method, params, 5000)
  local out = {}
  if r then
    for _, resp in pairs(r) do
      if resp.result then table.insert(out, vim.inspect(resp.result))
      elseif resp.error then table.insert(out, 'ERR: ' .. vim.inspect(resp.error))
      else table.insert(out, 'nil') end
    end
  else
    table.insert(out, 'TIMEOUT')
  end
  return table.concat(out, '\n')
end

vim.wait(5000, function() return vim.lsp.get_clients({name='paslsp'})[1] ~= nil end)
local c = vim.lsp.get_clients({name='paslsp'})[1]
f:write("LSP: " .. tostring(c and c.name or 'nil') .. "\n\n")

-- diagnostics
local diags = vim.diagnostic.get(0)
f:write("Diagnostics: " .. #diags .. "\n")
for i, d in ipairs(diags) do
  local lsp = d.user_data and d.user_data.lsp
  if lsp then
    f:write(string.format("  %d. %s (line %d, sev %d)\n", i, lsp.message, lsp.range.start.line + 1, lsp.severity))
  end
end
f:write("\n")

-- hover on 'add' declaration (line 6, col 10 = 'a' in "add")
f:write("=== HOVER on 'add' ===\n")
f:write(req('textDocument/hover', {
  textDocument = { uri = uri },
  position = { line = 6, character = 10 },
}) .. "\n\n")

-- hover on 'x' usage (line 17, col 5 = 'x')
f:write("=== HOVER on 'x' ===\n")
f:write(req('textDocument/hover', {
  textDocument = { uri = uri },
  position = { line = 17, character = 5 },
}) .. "\n\n")

-- hover on 'add' in middle of word (line 19, col 6 = second 'd')
f:write("=== HOVER on 'add' mid-word ===\n")
f:write(req('textDocument/hover', {
  textDocument = { uri = uri },
  position = { line = 19, character = 6 },
}) .. "\n\n")

-- definition on 'add' call (line 19, col 4 = 'a' in "add")
f:write("=== DEFINITION on 'add' call ===\n")
f:write(req('textDocument/definition', {
  textDocument = { uri = uri },
  position = { line = 19, character = 4 },
}) .. "\n\n")

-- references on 'x' (line 17, col 5)
f:write("=== REFERENCES on 'x' ===\n")
f:write(req('textDocument/references', {
  textDocument = { uri = uri },
  position = { line = 17, character = 5 },
  context = { includeDeclaration = true },
}) .. "\n\n")

-- document symbols
f:write("=== DOCUMENT SYMBOLS ===\n")
f:write(req('textDocument/documentSymbol', {
  textDocument = { uri = uri },
}) .. "\n\n")

-- completion
f:write("=== COMPLETION ===\n")
f:write(req('textDocument/completion', {
  textDocument = { uri = uri },
  position = { line = 0, character = 8 },
}) .. "\n\n")

f:close()
print("done")
